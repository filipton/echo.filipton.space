export type HttpRequest = {
    url: string;
    method: string;
    version: string;
    headers: Map<string, string>;
    body: string;

    raw: string;
};

export function parseRequest(raw: string): HttpRequest {
    const [requestLine, ...headersAndBody] = raw.split("\r\n");
    const [method, url, version] = requestLine.split(" ");

    const headers = new Map<string, string>();
    for (const line of headersAndBody) {
        if (line === "") {
            break;
        }

        const [key, value] = line.split(": ");
        headers.set(key, value);
    }

    let body = headersAndBody.slice(headers.size + 1).join("\r\n");

    return {
        url,
        method,
        version,
        headers,
        body,
        raw,
    };
}
