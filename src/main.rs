pub mod core;
use std::error::Error;

use crate::core::Config;

use wayland_client::Attached;
use wayland_client::EventQueue;

pub(crate) use crate::core as Nadva;

mod consts;
pub(crate) use crate::consts as CONST;

fn main() -> Result<(), Box<dyn Error>> {
    {
        use wayland_client::{Display, GlobalManager, protocol::wl_display::WlDisplay,};
        // Connect to the server
        let display: Display = Display::connect_to_env().unwrap();
    
        // Create the event queue
        let mut event_queue:EventQueue = display.create_event_queue();
        // Attach the display
        let attached_display: Attached<WlDisplay> = display.attach(event_queue.token());
    
        let globals: GlobalManager = GlobalManager::new(&attached_display);
    
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

    let config: Config = Nadva::Config::from_file()?;
    println!("{config:?}");

    Ok(())
}
