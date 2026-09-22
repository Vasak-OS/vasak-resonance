# Herramientas de medición

Lo que hay acá **no se empaqueta ni corre en la aplicación**: son bancos para
contestar preguntas de rendimiento con números en vez de con una corazonada.

## Por qué existen

El [#34](https://github.com/Vasak-OS/vasak-resonance/issues/34) sospechaba que
el reproductor se sentía tosco por cómo dibuja su tira de barras, y no se pudo
sostener con números. Los dos intentos fallaron por el mismo tipo de motivo:

1. El primero forzaba `getBoundingClientRect()` en cada cuadro. Eso le impone a
   `transform` un recálculo que no necesitaba, así que aplana la diferencia que
   se quería ver — y dio el resultado al revés.
2. El segundo miraba los intervalos entre cuadros, que es lo correcto, pero
   nunca corrió: con la página escondida `requestAnimationFrame` no se
   despierta y la medición se queda esperando para siempre.

`medir-en-webkit.py` arregla las dos cosas: abre una **ventana de verdad** —así
los cuadros existen— y lo hace en **WebKitGTK**, que es el motor donde se dibuja
la aplicación. Medir en otro motor contesta otra pregunta: Chromium y WebKit no
pagan lo mismo por un `height` en línea.

## Cómo se corre

Hace falta `python-gobject` y `webkit2gtk-4.1`, que ya son dependencias del
sistema en VasakOS.

```bash
python3 herramientas/medir-en-webkit.py herramientas/bancos/tira-de-barras.html
python3 herramientas/medir-en-webkit.py herramientas/bancos/cadencia-y-listas.html
```

Cada banco imprime JSON. Abre una ventana mientras mide: es a propósito, y se
cierra sola al terminar.

## Qué mide cada uno

- **`tira-de-barras.html`** — `height` contra `transform: scaleY()` sobre la
  tira del reproductor, escribiendo en **cada cuadro**. Es un techo, no lo que
  pasa: sirve para ver si el mecanismo se separa alguna vez. Sube la carga a
  330 y 1100 barras justamente para eso.
- **`cadencia-y-listas.html`** — la **cadencia real** del reproductor, que
  escribe dos veces por segundo y no sesenta, separando el cuadro que carga el
  tic de los demás; y lo que cuesta armar y desplazar una lista sin virtualizar,
  de 60 a 8000 filas.

## Al tocar el reproductor, tocar el banco

`tira-de-barras.html` y `cadencia-y-listas.html` repiten a mano la fórmula de
las alturas de `PlaybackWaves.vue`. Si esa fórmula cambia y el banco no, el
banco mide otra cosa y nadie se entera. Hay una prueba que los compara:
`tests/el-banco-mide-lo-que-dibuja.test.ts`.
