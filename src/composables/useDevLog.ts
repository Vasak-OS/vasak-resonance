const isDev = import.meta.env.DEV;

export const devLog = (...args: unknown[]) => {
	if (isDev) {
		console.log(...args);
	}
};
