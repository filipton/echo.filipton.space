<script lang="ts">
    import { dev } from "$app/environment";
    import { page } from "$app/stores";
    import { onDestroy, onMount } from "svelte";
    import { parseRequest, type HttpRequest } from "../lib/utils";

    let webSocket: WebSocket;
    let pingTimeout: number;
    let wsDestroyed = false;

    let wsScheme = $page.url.protocol === "https:" ? "wss" : "ws";
    let wsServer = dev ? "localhost:8080" : $page.url.host;

    let clientParams = "";
    let clientId = -1n;
    let requests: HttpRequest[] = [];

    let requestUrl: string;

    onMount(async () => {
        if ($page.url.search.length > 0) {
            let tmpClientId = BigInt($page.url.search.slice(1));

            if (tmpClientId > 0) {
                clientParams = `?${tmpClientId}`;
            }
        }

        await connectWs();
    });

    async function connectWs() {
        webSocket = new WebSocket(
            `${wsScheme}://${wsServer}/ws${clientParams}`
        );
        webSocket.binaryType = "arraybuffer";

        webSocket.onopen = () => {
            console.log("WebSocket connected");

            pingTimeout = setTimeout(ping, 10000);
        };

        webSocket.onmessage = (event) => {
            if (typeof event.data === "object") {
                let dataView = new DataView(event.data);
                clientId = dataView.getBigUint64(0, false);

                window.history.replaceState(null, "", `?${clientId}`);

                if (dev) {
                    requestUrl = `http://localhost:8080/r${clientId}`;
                } else {
                    requestUrl = `${$page.url.origin}/r${clientId}`;
                }

                return;
            } else {
                let request: HttpRequest = parseRequest(event.data);

                requests = [request, ...requests];
            }
        };

        webSocket.onclose = () => {
            console.log("WebSocket disconnected");

            clearTimeout(pingTimeout);
            if (!wsDestroyed) setTimeout(connectWs, 100);
        };
    }

    onDestroy(() => {
        wsDestroyed = true;
        if (webSocket) webSocket.close();
    });

    function ping() {
        webSocket.send("ping");
        pingTimeout = setTimeout(ping, 10000);
    }
</script>

<h1 class="text-4xl font-bold">Requests:</h1>
<p>
    Use <a class="font-bold underline" href={requestUrl}>{requestUrl}</a> as your
    request url!
</p>

{#each requests as request}
    <p class="text-xl my-2 border-black border-t-2 border-b-2 border-solid">
        {request.method}
        {request.url}
        {request.version}

        {#each request.headers as header}
            <p class="text-lg">
                {header[0]}: {header[1]}
            </p>
        {/each}

        {#if request.body}
            <p class="text-lg mt-2">
                {request.body}
            </p>
        {/if}
    </p>
{/each}
