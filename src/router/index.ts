import { createRouter, createWebHistory } from 'vue-router';

export const router = createRouter({
	history: createWebHistory(),
	routes: [
		{
			path: '/',
			component: () => import('@/layouts/WindowAppLayout.vue'),
			children: [
				{
					path: '',
					name: 'home',
					component: () => import('@/components/views/HomeView.vue'),
				},
				{
					path: 'albums',
					name: 'albums',
					component: () => import('@/components/views/AlbumsView.vue'),
				},
				{
					path: 'favorites',
					name: 'favorites',
					component: () => import('@/components/views/FavoritesView.vue'),
				},
				{
					path: 'playlists',
					name: 'playlists',
					component: () => import('@/components/views/PlaylistsView.vue'),
				},
			],
		},
		{
			path: '/:pathMatch(.*)*',
			redirect: { name: 'home' },
		},
	],
});
