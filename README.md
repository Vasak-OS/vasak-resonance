# Tauri + Vue + TypeScript

This template should help get you started developing with Vue 3 and TypeScript in Vite. The template uses Vue 3 `<script setup>` SFCs, check out the [script setup docs](https://v3.vuejs.org/api/sfc-script-setup.html#sfc-script-setup) to learn more.

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Volar](https://marketplace.visualstudio.com/items?itemName=Vue.volar) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

## Type Support For `.vue` Imports in TS

Since TypeScript cannot handle type information for `.vue` imports, they are shimmed to be a generic Vue component type by default. In most cases this is fine if you don't really care about component prop types outside of templates. However, if you wish to get actual prop types in `.vue` imports (for example to get props validation when using manual `h(...)` calls), you can enable Volar's Take Over mode by following these steps:

1. Run `Extensions: Show Built-in Extensions` from VS Code's command palette, look for `TypeScript and JavaScript Language Features`, then right click and select `Disable (Workspace)`. By default, Take Over mode will enable itself if the default TypeScript extension is disabled.
2. Reload the VS Code window by running `Developer: Reload Window` from the command palette.

You can learn more about Take Over mode [here](https://github.com/johnsoncodehk/volar/discussions/471).

## Las letras

La letra de un tema se busca primero **en el disco**, y sólo si no está se sale a
LRCLIB. En orden:

1. Un archivo `.lrc` con el mismo nombre que el de audio, al lado. Es el formato
   con marcas de tiempo, así que la letra sigue a la canción.
2. La letra guardada en la etiqueta del propio archivo, que es donde la dejan los
   etiquetadores. Si trae marcas de tiempo también sigue a la canción.
3. Un `.txt` con el mismo nombre, sin tiempos.

O sea que una biblioteca prolija tiene letras **sin conexión**, y que un `.lrc`
puesto a mano —el de la versión en vivo, el traducido, el que corrige lo que el
servicio tiene mal— le gana a lo que venga de la red. Editarlo se ve enseguida:
lo que sale del disco no se cachea.

Un archivo que no esté en UTF-8 se deja pasar en vez de adivinar la codificación,
y la letra sale de la red como antes.

## Las tapas

La tapa de un tema sale de la etiqueta del archivo, y si no la trae, de la
imagen que esté **al lado**, en la misma carpeta: `cover.jpg`, `cover.png`,
`folder.jpg`, `folder.png`, `front.jpg` o `album.jpg`, en ese orden y sin
importar cómo estén escritas las mayúsculas. Es la que dejan los ripeadores y
casi todas las descargas, y es la única fuente que funciona sin conexión y sin
equivocarse: la puso quien armó la carpeta.

Una imagen suelta con cualquier otro nombre **no** se toma como tapa. Ahí es
donde empiezan las equivocadas —la foto del grupo, el escaneo del librito—.

Recién cuando no hay ninguna de las dos se sale a buscarla a Deezer o a Cover
Art Archive, que es lo que necesita red y lo que puede traer la de otro disco.

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

**La tapa del disco** la dibuja Discord desde su lado, así que sólo le sirven
direcciones web: la imagen incrustada en el archivo no la puede ver. Lo que se le
manda es la dirección de la portada del álbum en Deezer o en Cover Art Archive,
que es de donde el reproductor ya baja las tapas que faltan; la dirección queda
anotada al lado de la imagen cacheada, así sigue estando la próxima vez aunque no
haya que volver a bajar nada. Si el álbum no está en ninguno de los dos, la
tarjeta sale con el logo del sistema como antes.

Esa búsqueda **sólo se hace con la presencia encendida**: le manda el artista y
el álbum a un servicio ajeno, y en un equipo sin identificador configurado no hay
ningún motivo para hacerlo.
