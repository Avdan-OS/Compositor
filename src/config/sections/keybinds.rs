use std::collections::HashMap;

use compositor_macros::export_test;
use json_tree::{Index, Location};
use serde::Deserialize;

use lazy_static::lazy_static;


use crate::{core::error::{TraceableError, Traceable}, config::templating::AvMacro};

use super::section::ConfigurationSection;

export_test!(
    Hello {
        "Move focused window to `R`th on the taskbar."
        window(d)       => (Super+{d}),

        "Toggle the active window into fullscreen mode."
        fullscreen()    => (Super+F),

        "Switch the focused workspace to workspace `{d}`."
        workspace(d)    => (Ctrl+Super+{d}),

        "Move the focused window to workspace `{d}`."
        moveWindowToWorkspace(d) => (Ctrl+Super+Shift+{d}),

        "Close the current window."
        closeWindow()   => (Alt+F4),

        "Open your default terminal."
        terminal()      => (Super+Enter),

        "Test String"
        bestFood()      => "Hawaiian (Pineapple) Pizza",

        "Test integer"
        bestNumber()    => 2,

        "Test float"
        bestFloat()    => 2.0,

        "List of many values (of different types)"
        listOStuff()    => ["apples", "flies", "oranges"]
    }
);

impl Hello {
    fn s(&self) {
        self.bestFloat;
    }
}


#[derive(Debug, Clone)]
pub struct Keybinds {
    variables : HashMap<String, String>,
    keybinds  : HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub struct KeybindsProto {
    multitasking: HashMap<String, String>,
}
lazy_static!{
    pub static ref MULTITASKING : HashMap<String, String> = {
        let h = HashMap::new();
        h
    };
}

impl ConfigurationSection for Keybinds {
    type Raw = KeybindsProto;
    const PATH : &'static str = "$.keybinds";


    fn parse(trace: Traceable, raw : &Self::Raw, index: &Index) -> Result<Self, Vec<Box<dyn TraceableError>>> {
        let abs = Self::path();
        let mut errors : Vec<Box<dyn TraceableError>> = vec![];

        println!("Multitasking section!");

        for (key, value) in raw.multitasking.iter() {
            let path = abs.push("multitasking").push(key.clone());

            let (key_loc, value_loc) = match index.get(&path).unwrap() {
                Location::KeyValue(k, v) => (k.loc(),v.loc()),
                Location::Value(_) => unreachable!() // Should never reach since we're a hash map.
            };
   
            let avmacro = match AvMacro::parse(
                trace.at_loc(key_loc), key.clone()
            ) {
                Ok(ok) => {
                    println!("{:?}", ok);
                    // Complete expansion process.
                },
                Err(er) => {
                    errors.extend(er);
                    continue;
                }
            };
        }

        if errors.len() > 0 {
            return Err(errors);
        }
        
        todo!();
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn deserialize_keybinds() {
        let raw = r#"{}"#;
    }
}