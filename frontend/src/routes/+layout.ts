import { redirect } from '@sveltejs/kit';
import type { LayoutLoad } from './$types';

export const load = (event => {
    if (event.url.protocol === 'http:') {
        // redirect to https
        const url = new URL(event.url);
        url.protocol = 'https:';

        throw redirect(301, url.href);
    }
}) satisfies LayoutLoad;
