#!/usr/bin/env python3
"""Corre una página de medición dentro de WebKitGTK y devuelve sus números.

**Por qué existe.** El reproductor se dibuja en WebKitGTK, y medir en otro motor
—el navegador que uno tenga a mano— contesta otra pregunta: Chromium y WebKit no
pagan lo mismo por un `height` en línea. Esto carga la página en el mismo motor
y la misma versión que usa la aplicación.

**Y por qué una ventana de verdad.** `requestAnimationFrame` no se despierta si
nadie está mirando: una medición con la ventana escondida se queda esperando
para siempre. Fue lo que dejó sin correr el segundo intento del issue #34.

Uso:  python3 herramientas/medir-en-webkit.py herramientas/bancos/tira.html
"""

import json
import pathlib
import sys

import gi

gi.require_version("Gtk", "3.0")
gi.require_version("WebKit2", "4.1")
from gi.repository import GLib, Gtk, WebKit2  # noqa: E402


def main() -> int:
    if len(sys.argv) < 2:
        print("falta la página a medir", file=sys.stderr)
        return 2

    pagina = pathlib.Path(sys.argv[1]).resolve()
    if not pagina.is_file():
        print(f"no existe: {pagina}", file=sys.stderr)
        return 2

    ventana = Gtk.Window(title="medición")
    # Del tamaño del reproductor: la cantidad de píxeles que hay que rehacer es
    # parte de lo que se está midiendo.
    ventana.set_default_size(1400, 700)

    vista = WebKit2.WebView()
    ventana.add(vista)

    salida: dict = {}

    def recibir(_gestor, resultado):
        salida["datos"] = json.loads(resultado.get_js_value().to_string())
        Gtk.main_quit()

    gestor = vista.get_user_content_manager()
    gestor.connect("script-message-received::resultado", recibir)
    gestor.register_script_message_handler("resultado")

    vista.load_uri(pagina.as_uri())
    ventana.show_all()

    # Un tope, para que una página rota no cuelgue la sesión.
    GLib.timeout_add_seconds(180, Gtk.main_quit)
    Gtk.main()

    if "datos" not in salida:
        print("la página no devolvió nada", file=sys.stderr)
        return 1

    print(json.dumps(salida["datos"], ensure_ascii=False, indent=1))
    return 0


if __name__ == "__main__":
    sys.exit(main())
