mod core;
use std::error::Error;

pub(crate) use crate::core as Nadva;

mod consts;
pub(crate) use crate::consts as CONST;

fn main() -> Result<(), Box<dyn Error>> {
    {
        use wayland_client::{Display, GlobalManager};
        
        // Connect to the server
        let display = Display::connect_to_env().unwrap();
    
        // Create the event queue
        let mut event_queue = display.create_event_queue();
        // Attach the display
        let attached_display = display.attach(event_queue.token());
    
        let globals = GlobalManager::new(&attached_display);
    
        event_queue.sync_roundtrip (
            &mut (),
            |_, _, _| unreachable!()
        ).unwrap();
    
        println!("Available globals:");
    
        for (name, interface, version) in globals.list() {
            println!("{name}: {interface} v{version}");
        }
        
        let config = config_loader::read_config()
            .unwrap();
        
        println!("{:#?}", config);
   }

    let config = Nadva::Config::from_file()?;
    println!("{config:?}");
    
    Ok(())
}

