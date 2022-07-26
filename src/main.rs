use wayland_client::{protocol::wl_registry, Connection, Dispatch, QueueHandle};

struct AppData;

impl Dispatch<wl_registry::WlRegistry, ()> for AppData {
    fn event (
        _state: &mut Self,
        _: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<AppData>,
    ) {
        if let wl_registry::Event::Global { name, interface, version } = event {
            println!("{name} {interface} ver{version}");
        } 
    }
}

fn main() {
   let connection = Connection::connect_to_env()
       .unwrap();

   let _display = connection.display();

   let mut event_queue = connection.new_event_queue();
   let _qh = event_queue.handle();

   println!("Advertized globals:");

   event_queue.roundtrip(&mut AppData)
       .unwrap();
}
