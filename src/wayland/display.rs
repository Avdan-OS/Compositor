use wayland_client::{
    Display,
    GlobalManager,
    protocol::wl_display::WlDisplay,
    Attached,
    EventQueue,
};

pub fn display_server() {
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
}
