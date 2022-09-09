use compositor_macros::config_section;

use crate::config::ConfigurationSection;

config_section!(
    Keybinds {
        "Move focused window to `d`th on the taskbar."
        window(d)       => (Meta+{d}),


use crate::{core::error::{TraceableError, Traceable}, config::{templating::AvMacro, Config}};

use super::section::ConfigurationSection;

export_test!(
    Keybinds {
        "Move focused window to `R`th on the taskbar."
        window(d)        => (Super+{d}),

        "Toggle the active window into fullscreen mode."
        full_screen()    => (Super+F),

        "Switch the focused workspace to workspace `{d}`."
        workspace(d)     => (Super+{d}),

        "Move the focused window to workspace `{d}`."
        move_window_to_workspace(d) => (Super+Shift+{d}),

        "Close the current window."
        close_window()   => (Super+Q),

        "Open your default terminal."
        terminal()       => (Super+Enter),

        "Test String"
        best_food()      => "Hawaiian (Pineapple) Pizza", // ! Akane is italian! You might get in trouble if he sees this!

        "Test integer"
        best_number()    => 2,

        "Test float"
        best_float()     => 2.0,

        "List of many values (of different types)"
        list_ostuff()    => ["apples", "flies", "oranges"]
    }
);

impl Keybinds {
    fn test(&self) {
        self.bestFloat;
    }
}

impl<'de> serde::de::Deserialize<'de> for Hello {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: serde::Deserializer<'de> 
    {
        let raw : HashMap<String, serde_json::Value> = Deserialize::deserialize(deserializer)?;

        let path : JSONPath = Self::PATH.to_string().try_into().unwrap(); 

        let res = raw.iter().map(|(k, v)| {
            // Parse as a macro.
            let p = path.push(k.clone());
            let loc = Config::index().get(&p).unwrap();
            
            (
                AvMacro::parse(
                    Traceable::combine(&Config::path(), loc, Some(true)),
                    k.clone()
                ),
                v
            )
        });

        let errors = res
            .filter(|(k, _)| k.is_err())
            .map(|(k, _)| k.unwrap_err());



        todo!()
        
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
    const PATH : &'static str = "$.keybinds";

    fn parse(trace: Traceable, raw : &Self::Raw, index: &Index) -> Result<Self, Vec<Box<dyn TraceableError>>> {
        let abs = Self::path();
        let index = Config::index();
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
    
}
