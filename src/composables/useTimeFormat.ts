export const formatSeconds = (value: number | null | undefined): string => {
	const safe = Math.max(0, Math.floor(value || 0));
	const minutes = Math.floor(safe / 60)
		.toString()
		.padStart(2, '0');
	const seconds = Math.floor(safe % 60)
		.toString()
		.padStart(2, '0');

	return `${minutes}:${seconds}`;
};
