use smithay::reexports::wayland_server::backend::{
    ClientData,
    ClientId,
    DisconnectReason,
};

pub struct ClientState;
impl ClientData for ClientState {
    fn initialized(&self, _client_id: ClientId) {
        println!("Client initialized.");
    }

    fn disconnected (
        &self,
        _client_id: ClientId,
        _reason: DisconnectReason
    ) {
        println!("Client disconnected.");
    }
}
