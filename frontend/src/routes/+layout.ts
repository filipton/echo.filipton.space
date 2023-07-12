import { redirect } from '@sveltejs/kit';
import type { LayoutLoad } from './$types';
import { dev } from '$app/environment';

export const load = (event => {
    if (event.url.protocol === 'http:' && !dev) {
        console.log('redirecting to https');
        // redirect to https
        const url = new URL(event.url);
        url.protocol = 'https:';

        throw redirect(301, url.href);
    }
}) satisfies LayoutLoad;
