<script lang="ts">
    import { dev } from "$app/environment";
    import { page } from "$app/stores";
    import { onMount } from "svelte";

    let webSocket: WebSocket;
    let wsScheme = $page.url.protocol === "https:" ? "wss" : "ws";
    let wsServer = dev ? "localhost:8080" : $page.url.host;

    let clientParams = "";
    let clientId = -1n;
    let requests: string[] = [];

    let requestUrl: string;

    onMount(async () => {
        if ($page.url.search.length > 0) {
            let tmpClientId = parseInt($page.url.search.slice(1));

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
        };

        webSocket.onmessage = (event) => {
            if (typeof event.data === "object") {
                let uint8Array = new Uint8Array(event.data);
                clientId = BigInt(
                    uint8Array.slice(0, 8).reduce((acc, cur) => {
                        return (acc << 8n) + BigInt(cur);
                    }, 0n)
                );

                window.history.replaceState(
                    null,
                    "",
                    `?${clientId}`
                );

                if (dev) {
                    requestUrl = `http://localhost:8080/r${clientId}`;
                } else {
                    requestUrl = `${$page.url.origin}/r${clientId}`;
                }

                return;
            } else {
                let requestStr = event.data;
                requests = [requestStr, ...requests];
            }
        };

        webSocket.onclose = () => {
            console.log("WebSocket disconnected");
            setTimeout(connectWs, 1000);
        };
    }
</script>

<h1 class="text-4xl font-bold">Requests:</h1>
<p>
    Use <a class="font-bold underline" href={requestUrl}>{requestUrl}</a> as your
    request url!
</p>

{#each requests as request}
    <p class="text-xl my-2 border-black border-t-2 border-b-2 border-solid">
        {request}
    </p>
{/each}
