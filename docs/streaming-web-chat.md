---
title: A tiny streaming web chat
description: Build a local browser chat on the Open Interpreter app-server protocol.
---

For a complete desktop, browser, or headless experience, use [Interpreter
Workstation](https://github.com/openinterpreter/interpreter-workstation) instead.
It already has a composer, approvals, sessions, and richer tool output. This
page is for learning how to put **your own small local UI** in front of raw
Open Interpreter (OIX), not for reproducing Workstation.

The browser talks to a loopback-only Node bridge; the bridge launches
`interpreter app-server --listen stdio://`. Your provider login/configuration
stays with the **server process**, never in JavaScript sent to the browser.
Sign in with `interpreter` first, then run these two files from a disposable
project directory. Node 20+ is needed; no npm packages are required.

The bridge sends JSON-RPC `initialize`, waits for its matching response, then
sends `initialized`. It calls `thread/start` once and `turn/start` for each
prompt. During that turn, `item/agentMessage/delta` carries text and
`turn/completed` carries the final status (including failures). The bridge
matches response IDs and thread/turn IDs before streaming text to the page.

Save the following two snippets as `bridge.mjs` and `index.html` in the same
directory, run `node bridge.mjs`, and open `http://127.0.0.1:8787/`.

<details>
<summary>Complete two-file local example</summary>

### `bridge.mjs`

```js
import { createServer } from "node:http";
import { spawn } from "node:child_process";
import { readFileSync } from "node:fs";
import { createInterface } from "node:readline";

const port = 8787;
const origin = `http://127.0.0.1:${port}`;
const child = spawn(process.env.OIX_BINARY ?? "interpreter",
  ["app-server", "--listen", "stdio://"], { stdio: ["pipe", "pipe", "inherit"] });
const pending = new Map();
let nextId = 0, threadId, active;
const write = message => child.stdin.write(JSON.stringify(message) + "\n");
function call(method, params) {
  const id = ++nextId;
  return new Promise((resolve, reject) => {
    pending.set(id, { resolve, reject });
    write({ id, method, params });
  });
}
function emit(a, event) {
  if (a.closed) return;
  const line = JSON.stringify(event) + "\n";
  a.bytes += Buffer.byteLength(line);
  if (a.bytes > 65536 || !a.res.write(line)) {
    a.res.end(); // bounded output: a slow reader cannot accumulate indefinitely
  }
}
function interrupt(a) {
  a.closed = true;
  if (a.turnId && !a.interrupted) {
    a.interrupted = true;
    call("turn/interrupt", { threadId, turnId: a.turnId }).catch(() => {});
  }
}
function notification(message) {
  const { method, params: p } = message;
  const a = active;
  if (!a || p?.threadId !== threadId) return;
  // Notifications may arrive before the turn/start *response*; queue only a
  // bounded number until its ID is known, then filter by both IDs.
  if (!a.turnId) {
    if (a.queue.length < 32) a.queue.push(message);
    else { a.res.end(); interrupt(a); }
    return;
  }
  if ((p.turnId ?? p.turn?.id) !== a.turnId) return;
  if (method === "item/agentMessage/delta") emit(a, { type: "text", text: p.delta });
  if (method === "error") emit(a, { type: "error", text: p.error.message });
  if (method === "turn/completed") {
    emit(a, { type: "done", status: p.turn.status,
      error: p.turn.error?.message ?? null });
    a.closed = true;
    a.res.end();
    active = undefined;
  }
}
createInterface({ input: child.stdout }).on("line", line => {
  let m;
  try { m = JSON.parse(line); } catch { return; }
  if (m.id !== undefined && m.method) {
    // This teaching example has no approval UI. Explicitly decline requests;
    // never accept one just because a browser is connected.
    const approval = ["item/commandExecution/requestApproval",
      "item/fileChange/requestApproval"].includes(m.method);
    write(approval ? { id: m.id, result: { decision: "decline" } }
      : { id: m.id, error: { code: -32601, message: "No UI for this request" } });
    if (active) emit(active, { type: "error", text: `Declined: ${m.method}` });
  } else if (m.id !== undefined) {
    const p = pending.get(m.id);
    if (p) { pending.delete(m.id); m.error ? p.reject(Error(m.error.message)) : p.resolve(m.result); }
  } else if (m.method) notification(m);
});
child.on("error", error => {
  console.error(`Cannot launch interpreter: ${error.message}`);
  for (const p of pending.values()) p.reject(error);
  pending.clear();
});
child.stdin.on("error", () => {}); // a failed child may close stdin first
child.on("exit", () => {
  for (const p of pending.values()) p.reject(Error("app server exited"));
  pending.clear();
  if (active) { emit(active, { type: "error", text: "app server exited" }); active.res.end(); }
  process.exitCode = 1;
});

// The app server requires the initialize response before initialized, then
// thread/start before turn/start. Keep a single read-only thread in this demo.
try {
  await call("initialize", { clientInfo: { name: "tiny-web-chat", version: "1.0" } });
  write({ method: "initialized" });
  threadId = (await call("thread/start", {
    cwd: process.cwd(), sandbox: "read-only", approvalPolicy: "on-request"
  })).thread.id;
} catch (error) {
  console.error(`App server start failed: ${error.message}`);
  process.exit(1);
}

const server = createServer(async (req, res) => {
  if (req.headers.host !== `127.0.0.1:${port}` ||
      (req.method === "POST" && req.headers.origin !== origin)) {
    res.writeHead(403).end(); return;
  }
  if (req.method === "GET" && req.url === "/") {
    res.writeHead(200, { "Content-Type": "text/html; charset=utf-8",
      "X-Content-Type-Options": "nosniff" });
    res.end(readFileSync(new URL("./index.html", import.meta.url))); return;
  }
  if (req.method !== "POST" || req.url !== "/chat") {
    res.writeHead(404).end(); return;
  }
  if (active) { res.writeHead(409).end("Wait for the current turn"); return; }
  if (req.headers["content-type"] !== "text/plain" ||
      Number(req.headers["content-length"] || 0) > 4096) {
    res.writeHead(413).end(); return;
  }
  const chunks = [];
  let size = 0;
  try {
    for await (const chunk of req) {
      size += chunk.length;
      if (size > 4096) throw Error("Prompt too long");
      chunks.push(chunk);
    }
  } catch { res.writeHead(413).end(); return; }
  const prompt = Buffer.concat(chunks).toString("utf8");
  if (!prompt.trim()) { res.writeHead(400).end(); return; }
  // Both bodies may have started while idle; only one may reserve the turn.
  if (active) { res.writeHead(409).end("Wait for the current turn"); return; }
  res.writeHead(200, { "Content-Type": "application/x-ndjson; charset=utf-8",
    "Cache-Control": "no-store", "X-Content-Type-Options": "nosniff" });
  const a = active = { res, queue: [], bytes: 0, closed: false };
  res.on("close", () => { if (active === a && !a.closed) interrupt(a); });
  try {
    a.turnId = (await call("turn/start", {
      threadId, input: [{ type: "text", text: prompt }]
    })).turn.id;
    if (a.closed) interrupt(a);
    for (const event of a.queue) notification(event);
    a.queue.length = 0;
  } catch (error) {
    emit(a, { type: "error", text: error.message });
    a.closed = true; res.end(); active = undefined;
  }
});
server.requestTimeout = 30_000;
server.listen(port, "127.0.0.1", () => console.log(origin));
```

### `index.html`

```html
<!doctype html>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>Local Open Interpreter chat</title>
<style>
  body { max-width: 48rem; margin: 3rem auto; padding: 0 1rem;
    font: 1rem/1.5 system-ui; color: #20242c; background: #fafafa }
  #chat { white-space: pre-wrap; min-height: 10rem; padding: 1rem;
    background: white; border: 1px solid #ddd; border-radius: .8rem }
  form { display: flex; gap: .6rem; margin-top: 1rem }
  textarea { flex: 1; min-height: 4rem; font: inherit }
</style>
<h1>Local chat</h1><div id="chat" role="log" aria-live="polite"></div>
<form><textarea aria-label="Your message" maxlength="4096" required></textarea>
  <button>Send</button></form>
<script>
const form = document.querySelector("form"), box = document.querySelector("#chat");
const input = form.querySelector("textarea"), button = form.querySelector("button");
form.addEventListener("submit", async event => {
  event.preventDefault(); button.disabled = true;
  box.textContent += `\nYou: ${input.value}\nAssistant: `;
  try {
    const response = await fetch("/chat", { method: "POST",
      headers: { "Content-Type": "text/plain" }, body: input.value });
    if (!response.ok) throw Error(`HTTP ${response.status}`);
    input.value = "";
    const reader = response.body.getReader(), decoder = new TextDecoder();
    let buffer = "";
    while (true) {
      const { value, done } = await reader.read();
      if (done) break;
      buffer += decoder.decode(value, { stream: true });
      let newline;
      while ((newline = buffer.indexOf("\n")) >= 0) {
        const event = JSON.parse(buffer.slice(0, newline));
        buffer = buffer.slice(newline + 1);
        if (event.type === "text") box.textContent += event.text;
        if (event.type === "error") box.textContent += `\n[${event.text}]`;
        if (event.type === "done" && event.status !== "completed")
          box.textContent += `\n[${event.status}: ${event.error ?? "no details"}]`;
      }
    }
  } catch (error) { box.textContent += `\n[${error.message}]`; }
  finally { box.textContent += "\n"; button.disabled = false; }
});
</script>
```

</details>

The same server home and working directory determine the account, provider
and workspace.
This deliberately supports **one local user and one turn at a time**; a second
POST gets `409`. It is not identity isolation: every local tab shares this
thread. Closing the response interrupts its turn; until the app server reports
`turn/completed`, another turn stays blocked. Each streamed
delta and completion is matched to the active thread and turn; JSON-RPC
responses are matched by request ID. Messages are rendered as text, never
HTML. This demo **declines** command/file approvals and rejects other server
requests; use Workstation or implement a real review UI if approvals are
needed. It does not display tool events, images, conversation history, or
multiple users.

**Security boundary:** keep the bridge on loopback. Do not forward its port or
expose the raw app-server socket publicly. The Host/Origin checks and small
input/output bounds above are educational safeguards, **not** remote
authentication, CSRF protection for every deployment, rate limiting,
permission isolation, or production backpressure handling. For remote or
multi-user hosting you must add authenticated transport, explicit user/session
isolation, authorization, resource limits, and proper approval handling; do
not put tokens, OAuth files, or provider keys into the page. See [App Server](/docs/app-server),
[Sandbox & approvals](/docs/sandbox), and [Workstation's public source](https://github.com/openinterpreter/interpreter-workstation)
for full-client design.
