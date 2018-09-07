
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate heck;

mod json;
mod classes;
mod methods;
mod special_methods;
mod documentation;

use std::fs::File;
use std::io::Write;
use std::collections::HashSet;

use json::*;
use classes::*;
use methods::*;
use special_methods::*;
use documentation::*;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Crate {
    Core,
    Graphics,
    Animation,
    Network,
    Audio,
    Video,
    ArVr,
    Input,
    Ui,
    Editor,
    VisualScript,
}

impl Crate {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "core" => Some(Crate::Core),
            "audio" => Some(Crate::Audio),
            "graphics" => Some(Crate::Graphics),
            "editor" => Some(Crate::Editor),
            "visual-script" => Some(Crate::VisualScript),
            _ => None,
        }
    }
}

pub fn generate_bindings(
    api_description: File,
    output: &mut File,
    crate_type: Option<Crate>,
) {

    let classes: Vec<GodotClass> = serde_json::from_reader(api_description)
        .expect("Failed to parse the API description");

    writeln!(output, "use std::os::raw::c_char;").unwrap();
    writeln!(output, "use std::ptr;").unwrap();
    //writeln!(output, "use std::mem;").unwrap();
    writeln!(output, "use object;").unwrap();

    for class in &classes {
        if let Some(crate_type) = crate_type {
            if crate_type != get_crate(&classes, class) {
                continue;
            }
        }

        generate_class_documentation(output, &classes, class);

        generate_class_struct(output, class);

        for e in &class.enums {
            generate_enum(output, class, e);
        }

        generate_method_table(output, class);

        generate_godot_object_impl(output, class);

        for method in &class.methods {
            generate_method_impl(output, class, method);
        }

        writeln!(output, "impl {} {{", class.name).unwrap();

        if class.singleton {
            generate_singleton_getter(output, class);
        }

        if class.instanciable {

            if class.is_refcounted() {
                generate_refreference_ctor(output, class);
            } else {
                generate_non_refreference_ctor(output, class);
            }
        }

        let mut method_set = HashSet::default();

        generate_methods(
            output,
            &classes,
            &mut method_set,
            &class.name,
            class.is_pointer_safe(),
            true,
        );

        generate_upcast(
            output,
            &classes,
            &class.base_class,
            class.is_pointer_safe(),
        );

        generate_dynamic_cast(output, class);

        writeln!(output, "}}").unwrap();

        if class.is_refcounted() && class.instanciable {
            generate_drop(output, class);
        }
    }
}

fn rust_safe_name(name: &str) -> &str {
    match name {
        "use" => "_use",
        "type" => "_type",
        "loop" => "_loop",
        "in" => "_in",
        "override" => "_override",
        "where" => "_where",
        name => name,
    }
}

pub fn find_class<'a, 'b>(classes: &'a[GodotClass], name: &'b str) -> Option<&'a GodotClass> {
    for class in classes {
        if &class.name == name {
            return Some(class);
        }
    }

    None
}

pub fn class_inherits(classes: &[GodotClass], class: &GodotClass, base_class_name: &str) -> bool {
    if class.base_class == base_class_name {
        return true;
    }

    if let Some(parent) = find_class(classes, &class.base_class) {
        return class_inherits(classes, parent, base_class_name);
    }

    return false;
}

fn get_crate(classes: &[GodotClass], class: &GodotClass) -> Crate {
    match &class.name[..] {
        "Shader"
        | "Texture"
        | "Viewport"
        | "InputEvent" | "InputEventKey" | "InputEventWithModifiers"
        | "NetworkedMultiplayerPeer" | "PacketPeer"
        | "Material" => {
            return Crate::Core;
        }
        _ => {}
    }

    if class.name.contains("VideoStream") {
        return Crate::Video;
    }

    if class.name.contains("PhysicsMaterial") {
        return Crate::Core;
    }

    if class.name.contains("ARVR") {
        return Crate::ArVr;
    }

    if class.name.contains("Audio") {
        return Crate::Audio;
    }

    if class.name.contains("Animation") || class.name.contains("Tween") {
        return Crate::Animation;
    }

    if class.name.contains("VisualScript") {
        return Crate::VisualScript;
    }

    if class.name.contains("InputEvent") {
        return Crate::Input;
    }

    if class.api_type == "tools" {
        return Crate::Editor;
    }

    if class.name.contains("Stream")
        || class.name.contains("WebSocket")
        || class.name.contains("Peer")
        || class.name.contains("HTTP")
        || class.name.contains("TCP")
        || class.name.contains("Network") {
        return Crate::Network;
    }

    if class.name.contains("VisualServer")
        || class.name.contains("Shader")
        || class.name.contains("Tile")
        || class.name.contains("Sprite")
        || class.name.contains("Material")
        || class.name.contains("Particle")
        || class.name.contains("CSG")
        || class.name.contains("GIProbe")
        || class.name.contains("Light")
        || class.name.contains("CubeMap")
        || class.name.contains("CubeMesh")
        //|| class.name.contains("Texture")
        //|| class.name.contains("Sky")
    {
        return Crate::Graphics;
    }

    if class_inherits(classes, class, "Control")
        || class_inherits(classes, class, "Popup")
        || class_inherits(classes, class, "Button")
        || class.name.contains("Button")
        || class.name == "GraphEdit" {
        return Crate::Ui;
    }

    Crate::Core
}
