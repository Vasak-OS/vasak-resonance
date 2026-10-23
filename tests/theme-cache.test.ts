/**
 * Recordar el tema entre arranques: la ventana no parpadea al iniciar.
 *
 * Se prueba la lógica con un elemento y un almacén de prueba (no los globales)
 * y, de fuente, que el cableado exista: el script del `<head>` y la llamada a
 * `saveTheme` tras cada `loadConfig` de `useConfigSync`.
 */

import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { applyTheme, saveTheme, THEME_KEY } from '@/tools/theme-cache';

const memoryStore = (initial: Record<string, string> = {}): Storage => {
	const data = new Map(Object.entries(initial));
	return {
		get length() {
			return data.size;
		},
		clear: () => data.clear(),
		getItem: (key: string) => data.get(key) ?? null,
		key: (index: number) => Array.from(data.keys())[index] ?? null,
		removeItem: (key: string) => void data.delete(key),
		setItem: (key: string, value: string) => void data.set(key, value),
	};
};

const throwingStore = (): Storage => {
	const fail = () => {
		throw new Error('localStorage no disponible');
	};
	return { length: 0, clear: fail, getItem: fail, key: fail, removeItem: fail, setItem: fail };
};

describe('theme-cache', () => {
	test('la clave es la que lee index.html', () => {
		expect(THEME_KEY).toBe('vsk-theme');
	});

	test('saveTheme guarda la clase oscura y sólo las variables --*', () => {
		const root = document.createElement('div');
		root.classList.add('dark');
		root.style.setProperty('--ui-bg', '#111');
		root.style.setProperty('color', 'red');
		const store = memoryStore();

		saveTheme(root, store);

		expect(JSON.parse(store.getItem(THEME_KEY) as string)).toEqual({
			dark: true,
			vars: { '--ui-bg': '#111' },
		});
	});

	test('applyTheme reaplica la clase oscura y las variables', () => {
		const root = document.createElement('div');
		const store = memoryStore({
			[THEME_KEY]: JSON.stringify({ dark: true, vars: { '--ui-bg': '#222' } }),
		});

		applyTheme(root, store);

		expect(root.classList.contains('dark')).toBe(true);
		expect(root.style.getPropertyValue('--ui-bg')).toBe('#222');
	});

	test('ida y vuelta: lo guardado es lo que se reaplica', () => {
		const first = document.createElement('div');
		first.classList.add('dark');
		first.style.setProperty('--primary', '#abc');
		const store = memoryStore();
		saveTheme(first, store);

		const second = document.createElement('div');
		applyTheme(second, store);

		expect(second.classList.contains('dark')).toBe(true);
		expect(second.style.getPropertyValue('--primary')).toBe('#abc');
	});

	test('un tema claro no agrega la clase oscura', () => {
		const root = document.createElement('div');
		applyTheme(root, memoryStore({ [THEME_KEY]: JSON.stringify({ dark: false, vars: {} }) }));
		expect(root.classList.contains('dark')).toBe(false);
	});

	test('sin clave guardada no hace nada', () => {
		const root = document.createElement('div');
		applyTheme(root, memoryStore());
		expect(root.className).toBe('');
		expect(root.getAttribute('style')).toBeNull();
	});

	test('un JSON inválido no lanza ni toca el elemento', () => {
		const root = document.createElement('div');
		expect(() => applyTheme(root, memoryStore({ [THEME_KEY]: '{no es json' }))).not.toThrow();
		expect(root.className).toBe('');
	});

	test('un almacén que lanza no rompe ni al guardar ni al aplicar', () => {
		const root = document.createElement('div');
		expect(() => saveTheme(root, throwingStore())).not.toThrow();
		expect(() => applyTheme(root, throwingStore())).not.toThrow();
	});
});

describe('cableado del tema recordado', () => {
	const read = (path: string) => readFileSync(new URL(`../${path}`, import.meta.url), 'utf8');

	test('index.html reaplica el tema antes de montar', () => {
		const html = read('index.html');
		expect(html).toContain('"vsk-theme"');
		expect(html).toContain('classList.add("dark")');
		expect(html).toContain('setProperty');
		expect(html.indexOf('"vsk-theme"')).toBeLessThan(html.indexOf('type="module"'));
	});

	test('useConfigSync guarda el tema tras cada loadConfig', () => {
		const source = read('src/composables/useConfigSync.ts');
		expect(source).toContain("from '@/tools/theme-cache'");
		const loads = source.match(/\.loadConfig\(\)/g)?.length ?? 0;
		const saves = source.match(/saveTheme\(\)/g)?.length ?? 0;
		expect(loads).toBe(3);
		expect(saves).toBe(loads);
	});
});
