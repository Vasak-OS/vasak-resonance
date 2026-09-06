import { getIconSource } from '@vasakgroup/plugin-vicons';
import { setupContextMenu } from '@vasakgroup/plugin-vsk-contextual-menu';
import I18n from '@vasakgroup/tauri-plugin-i18n';
import { createPinia } from 'pinia';
import { createApp } from 'vue';
import App from '@/App.vue';
import { sanearUrl } from '@/tools/csp';
import '@/assets/main.css';
import { captureFailures } from '@vasakgroup/plugin-vsk-journal';
import { router } from '@/router';

// Una violación de CSP no se ve: el recurso no carga y la interfaz queda a
// medias sin decir nada. Se sanean **las dos** URLs, porque `sourceFile` también
// puede llevar query con datos sensibles.
document.addEventListener('securitypolicyviolation', (evento) => {
	// El respaldo va **después** de sanear, no antes.
	//
	// Mirando el valor crudo, una entrada como `?token=X` es verdadera y
	// pasa el respaldo de largo — pero lo que queda de ella al sanearla es
	// nada, así que el registro salía con el campo en blanco. Sanear
	// primero y decidir después es lo que hace que un aviso incompleto no
	// exista.
	const recurso = sanearUrl(evento.blockedURI) || '(en línea)';
	const origen = sanearUrl(evento.sourceFile) || 'documento';
	console.error(
		`[CSP] bloqueado ${recurso} por la directiva ` +
			`«${evento.violatedDirective}» en ${origen}:${evento.lineNumber}`
	);
});

// El clic derecho abre el menú de VasakOS —el mismo de todo el escritorio— y no
// el del motor del navegador, que ofrecía «Recargar» e «Inspeccionar elemento»
// y cortaba la canción a mitad.
setupContextMenu({ iconResolver: getIconSource });

const i18n = I18n.getInstance();
// Lo que rompe la interfaz va al diario del sistema, con el nombre de esta
// aplicación. Antes no iba a ninguna parte: un error de JavaScript deja la
// pantalla a medias y la consola del WebView no la ve nadie en una máquina
// instalada.
captureFailures();

const app = createApp(App);
const pinia = createPinia();

app.use(pinia);
app.use(router);

// Se espera a que estén las traducciones antes de mostrar nada: montando
// primero, el arranque enseñaba las claves crudas —«contextMenu.play» y
// compañía— hasta que el archivo de idioma terminaba de cargar.
await i18n.load();

app.mount('#app');
