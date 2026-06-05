use gtk_layer_shell::{Edge, Layer, LayerShell};

pub fn setup_mini_player(gtk_window: gtk::ApplicationWindow) -> bool {
    if !gtk_layer_shell::is_supported() {
        eprintln!("[layer_shell] wlr-layer-shell no soportado por el compositor");
        return false;
    }

    gtk_window.init_layer_shell();
    gtk_window.set_layer(Layer::Top);
    gtk_window.set_namespace("vasak-resonance-miniplayer");

    gtk_window.set_anchor(Edge::Bottom, true);
    gtk_window.set_anchor(Edge::Right, true);

    gtk_window.set_layer_shell_margin(Edge::Bottom, 10);
    gtk_window.set_layer_shell_margin(Edge::Right, 10);

    true
}
