<script lang="ts">
    import { browser, dev } from "$app/environment";
    import { page } from "$app/stores";
    import "../app.css";

    if (browser && !dev) {
        if ($page.url.protocol == "http:") {
            window.location.href = `https://${$page.url.host}${$page.url.pathname}${$page.url.search}`;
        }
    }
</script>

<slot />
