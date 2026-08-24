# Tauri + Vue + TypeScript

This template should help get you started developing with Vue 3 and TypeScript in Vite. The template uses Vue 3 `<script setup>` SFCs, check out the [script setup docs](https://v3.vuejs.org/api/sfc-script-setup.html#sfc-script-setup) to learn more.

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Volar](https://marketplace.visualstudio.com/items?itemName=Vue.volar) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

## Type Support For `.vue` Imports in TS

Since TypeScript cannot handle type information for `.vue` imports, they are shimmed to be a generic Vue component type by default. In most cases this is fine if you don't really care about component prop types outside of templates. However, if you wish to get actual prop types in `.vue` imports (for example to get props validation when using manual `h(...)` calls), you can enable Volar's Take Over mode by following these steps:

1. Run `Extensions: Show Built-in Extensions` from VS Code's command palette, look for `TypeScript and JavaScript Language Features`, then right click and select `Disable (Workspace)`. By default, Take Over mode will enable itself if the default TypeScript extension is disabled.
2. Reload the VS Code window by running `Developer: Reload Window` from the command palette.

You can learn more about Take Over mode [here](https://github.com/johnsoncodehk/volar/discussions/471).

## La presencia en Discord

Resonance puede mostrar en tu perfil de Discord lo que estás escuchando: el
título, el artista, la barra con lo que falta, y la tapa del disco cuando hay una
que Discord pueda ver.

Hace falta una aplicación de Discord propia, porque la presencia se asocia a una:

1. Creá una aplicación en <https://discord.com/developers/applications>.
2. En **Rich Presence → Art Assets**, subí dos imágenes con estos nombres:
   `vasakos` (el logo del sistema, que se usa cuando no hay tapa) y `pausa` (el
   icono chico que se superpone con la música detenida).
3. Poné el identificador de esa aplicación en la configuración:

   ```json
   { "resonance": { "discord_app_id": "123456789012345678" } }
   ```

   en `~/.config/vasak/vasak.conf`, o en la variable de entorno
   `VASAK_DISCORD_APP_ID`, que gana sobre la anterior y sirve para probar.

Sin identificador la función queda apagada y el reproductor no cambia en nada.
Si Discord no está abierto tampoco pasa nada: se reintenta cada tanto, y el que
espera es un hilo aparte, nunca la interfaz ni el audio.

**La tapa del disco sale del archivo**, así que Discord no la puede ver: dibuja
la imagen desde su lado y sólo llega a direcciones web. Mientras las tapas sean
locales, se muestra el logo del sistema.
