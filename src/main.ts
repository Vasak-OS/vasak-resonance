import { getIconSource } from '@vasakgroup/plugin-vicons';
import { setupContextMenu } from '@vasakgroup/plugin-vsk-contextual-menu';
import I18n from '@vasakgroup/tauri-plugin-i18n';
import { createPinia } from 'pinia';
import { createApp } from 'vue';
import App from '@/App.vue';
import '@/assets/main.css';
import { router } from '@/router';

// Una violación de CSP no se ve: el recurso simplemente no carga y la interfaz
// queda a medias sin decir nada. Esto la manda a la consola, que es donde se
// puede encontrar al ajustar la política.
document.addEventListener('securitypolicyviolation', (evento) => {
	console.error(
		`[CSP] bloqueado ${evento.blockedURI || '(en línea)'} por la directiva ` +
			`«${evento.violatedDirective}» en ${evento.sourceFile ?? 'documento'}:${evento.lineNumber}`
	);
});

// El clic derecho abre el menú de VasakOS —el mismo de todo el escritorio— y no
// el del motor del navegador, que ofrecía «Recargar» e «Inspeccionar elemento»
// y cortaba la canción a mitad.
setupContextMenu({ iconResolver: getIconSource });

const i18n = I18n.getInstance();
const app = createApp(App);
const pinia = createPinia();

app.use(pinia);
app.use(router);

// Se espera a que estén las traducciones antes de mostrar nada: montando
// primero, el arranque enseñaba las claves crudas —«contextMenu.play» y
// compañía— hasta que el archivo de idioma terminaba de cargar.
await i18n.load();

app.mount('#app');
