import { createHmac, randomBytes, timingSafeEqual } from "node:crypto";
import { promises as fs } from "node:fs";
import {
  createServer,
  type IncomingMessage,
  type Server,
  type ServerResponse,
} from "node:http";
import { tmpdir } from "node:os";
import {
  dirname,
  extname,
  isAbsolute,
  join,
  relative,
  resolve,
} from "node:path";
import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";

const PROTOCOL_VERSION = 1;
const HOST = "127.0.0.1";
const MAX_EVENT_BYTES = 128 * 1024;
const MAX_SOURCE_BYTES = 1024 * 1024;
const MAX_SOURCE_LINE = 1_000_000;
const MAX_SOURCE_COLUMN = 10_000;
const BASE64URL_RE = /^[A-Za-z0-9_-]+$/;

interface ActiveAdapter {
  server: Server;
  projectRoot: string;
  metadataPath: string;
  metadataPathFromOverride: boolean;
  origin: string;
  token: string;
  sourceHintSecret: string;
  startedAt: string;
  events: number;
}

interface CommandUi {
  setStatus(key: string, value: string | undefined): void;
  setWidget(key: string, value: string[] | undefined): void;
  notify(message: string, level: "info" | "warning" | "error"): void;
}

interface CommandContext {
  cwd: string;
  ui: CommandUi;
}

interface BrowserIterateEvent {
  version: number;
  token: string;
  prompt: string;
  page?: {
    url?: string;
    title?: string;
  };
  target?: {
    tag?: string;
    id?: string;
    classes?: string[];
    role?: string;
    href?: string;
    text?: string;
    accessibleName?: string;
    selector?: string;
    input?: {
      type?: string;
      name?: string;
      placeholder?: string;
      checked?: boolean;
      disabled?: boolean;
      required?: boolean;
    };
    labels?: string[];
    aria?: Record<string, string>;
    landmark?: {
      tag?: string;
      role?: string;
      label?: string;
      selector?: string;
      heading?: string;
    };
    form?: {
      action?: string;
      method?: string;
      formAction?: string;
      formMethod?: string;
      selector?: string;
      fields?: string[];
      submitText?: string[];
      submitter?: {
        selector?: string;
        action?: string;
        method?: string;
      };
    };
    htmlExcerpt?: string;
  };
  cheers?: {
    source?: string;
    generatedId?: string;
    datastar?: Record<string, string>;
    datastarContext?: Array<{
      tag?: string;
      selector?: string;
      attributes?: Record<string, string>;
    }>;
  };
}

interface RustSourceContext {
  location: string;
  snippet: string;
}

let active: ActiveAdapter | undefined;
let commandQueue: Promise<void> = Promise.resolve();

function runCommandExclusive(action: () => Promise<void>): Promise<void> {
  const next = commandQueue.then(action, action);
  commandQueue = next.catch(() => undefined);
  return next;
}

function json(value: unknown): string {
  return JSON.stringify(value, null, 2);
}

function normalizeProjectPath(projectRoot: string): string {
  const normalized = projectRoot.replace(/\\/g, "/").replace(/\/+$/g, "");
  return normalized || projectRoot;
}

async function canonicalProjectRoot(cwd: string): Promise<string> {
  try {
    return await fs.realpath(cwd);
  } catch {
    return cwd;
  }
}

function cheersIterateRuntimeDir(): string {
  const xdgRuntimeDir = process.env.XDG_RUNTIME_DIR?.trim();
  return xdgRuntimeDir || tmpdir();
}

function cheersIterateMetadataDir(): string {
  return join(cheersIterateRuntimeDir(), "cheers", "iterate");
}

interface MetadataPath {
  path: string;
  fromOverride: boolean;
}

function resolveMetadataPath(token: string): MetadataPath {
  const override = process.env.CHEERS_ITERATE_METADATA?.trim();
  if (override) return { path: override, fromOverride: true };

  const safeName = `${process.pid}-${Date.now().toString(36)}-${token.slice(0, 12)}.json`;
  return {
    path: join(cheersIterateMetadataDir(), safeName),
    fromOverride: false,
  };
}

function oneLine(value: unknown): string | undefined {
  if (typeof value !== "string") return undefined;
  const trimmed = value.replace(/\s+/g, " ").trim();
  return trimmed || undefined;
}

function numberLine(value: unknown): string | undefined {
  return typeof value === "number" && Number.isFinite(value)
    ? String(Math.round(value))
    : undefined;
}

function pushField(lines: string[], label: string, value: unknown): void {
  const text = typeof value === "number" ? numberLine(value) : oneLine(value);
  if (text) lines.push(`- ${label}: ${text}`);
}

function fenced(label: string, value: unknown, language = "html"): string[] {
  const text = typeof value === "string" ? value.trim() : "";
  if (!text) return [];
  return [`${label}:`, `\`\`\`${language}`, text, "```"];
}

function parseRustSourceLocation(
  source: unknown,
): { file: string; line: number; column: number } | undefined {
  const text = typeof source === "string" ? source.trim() : "";
  if (!text) return undefined;

  const match = /^(.*):(\d+):(\d+)$/.exec(text);
  if (!match?.[1]) return undefined;

  const line = Number(match[2]);
  const column = Number(match[3]);
  if (
    !Number.isSafeInteger(line) ||
    !Number.isSafeInteger(column) ||
    line < 1 ||
    column < 1 ||
    line > MAX_SOURCE_LINE ||
    column > MAX_SOURCE_COLUMN
  ) {
    return undefined;
  }

  return { file: match[1], line, column };
}

function decodeBase64UrlUtf8(value: string): string | undefined {
  if (!BASE64URL_RE.test(value)) return undefined;

  const decoded = Buffer.from(value, "base64url");
  const text = decoded.toString("utf8");
  return Buffer.from(text, "utf8").toString("base64url") === value
    ? text
    : undefined;
}

function verifyRustSourceHint(
  source: unknown,
  sourceHintSecret: string,
): string | undefined {
  const text = typeof source === "string" ? source.trim() : "";
  if (!text) return undefined;

  const [version, payload, signature, extra] = text.split(".");
  if (version !== "v1" || !payload || !signature || extra !== undefined)
    return undefined;
  if (!BASE64URL_RE.test(payload) || !BASE64URL_RE.test(signature))
    return undefined;

  const expected = createHmac("sha256", sourceHintSecret)
    .update("v1.")
    .update(payload)
    .digest();
  const actual = Buffer.from(signature, "base64url");
  if (actual.length !== expected.length || !timingSafeEqual(actual, expected))
    return undefined;

  const decoded = decodeBase64UrlUtf8(payload);
  return parseRustSourceLocation(decoded) ? decoded : undefined;
}

function isWithinProject(projectRoot: string, path: string): boolean {
  const rel = relative(projectRoot, path);
  return rel === "" || (!!rel && !rel.startsWith("..") && !isAbsolute(rel));
}

function formatSourceSnippet(
  contents: string,
  line: number,
  column: number,
): string {
  const lines = contents.split(/\r?\n/);
  if (line > lines.length) return "";

  const start = Math.max(1, line - 4);
  const end = Math.min(lines.length, line + 4);
  const width = String(end).length;
  const output: string[] = [];

  for (let current = start; current <= end; current += 1) {
    const marker = current === line ? ">" : " ";
    output.push(
      `${marker} ${String(current).padStart(width, " ")} | ${lines[current - 1] ?? ""}`,
    );
    if (current === line) {
      output.push(
        `  ${" ".repeat(width)} | ${" ".repeat(Math.max(0, column - 1))}^`,
      );
    }
  }

  return output.join("\n");
}

async function readRustSourceContext(
  source: unknown,
  projectRoot: string,
): Promise<RustSourceContext | undefined> {
  const parsed = parseRustSourceLocation(source);
  if (!parsed) return undefined;

  try {
    const root = projectRoot;
    const candidate = isAbsolute(parsed.file)
      ? parsed.file
      : resolve(root, parsed.file);
    const realPath = await fs.realpath(candidate);
    if (!isWithinProject(root, realPath)) return undefined;
    if (extname(realPath) !== ".rs") return undefined;

    const stat = await fs.stat(realPath);
    if (!stat.isFile() || stat.size > MAX_SOURCE_BYTES) return undefined;

    const contents = await fs.readFile(realPath, "utf8");
    const snippet = formatSourceSnippet(contents, parsed.line, parsed.column);
    if (!snippet) return undefined;

    const displayPath = relative(root, realPath) || ".";
    return {
      location: `${displayPath.replace(/\\/g, "/")}:${parsed.line}:${parsed.column}`,
      snippet,
    };
  } catch {
    return undefined;
  }
}

function formatAgentMessage(
  event: BrowserIterateEvent,
  rustSource?: RustSourceContext,
  sourceHint?: string,
): string {
  const lines: string[] = [
    "Browser-initiated Cheers iteration request.",
    "",
    "User request:",
    event.prompt.trim(),
    "",
    "Clicked target:",
  ];

  pushField(lines, "URL", event.page?.url);
  pushField(lines, "Title", event.page?.title);
  pushField(lines, "Element", event.target?.tag);
  pushField(lines, "Text", event.target?.text);
  pushField(lines, "Accessible name", event.target?.accessibleName);
  pushField(lines, "Role", event.target?.role);
  pushField(lines, "ID", event.target?.id);
  if (event.target?.classes?.length)
    lines.push(`- Classes: ${event.target.classes.join(" ")}`);
  pushField(lines, "Selector", event.target?.selector);
  pushField(lines, "Href", event.target?.href);
  pushField(lines, "Input type", event.target?.input?.type);
  pushField(lines, "Input name", event.target?.input?.name);
  pushField(lines, "Input placeholder", event.target?.input?.placeholder);
  if (event.target?.input) {
    const flags = [
      event.target.input.required ? "required" : undefined,
      event.target.input.disabled ? "disabled" : undefined,
      event.target.input.checked ? "checked" : undefined,
    ].filter(Boolean);
    if (flags.length) lines.push(`- Input flags: ${flags.join(", ")}`);
  }
  if (event.target?.labels?.length)
    lines.push(`- Labels: ${event.target.labels.join(" | ")}`);
  if (event.target?.aria && Object.keys(event.target.aria).length) {
    lines.push(`- ARIA attrs: ${json(event.target.aria)}`);
  }

  const domLines: string[] = [];
  if (event.target?.landmark) {
    const landmark = [
      event.target.landmark.tag,
      event.target.landmark.role
        ? `role=${event.target.landmark.role}`
        : undefined,
      event.target.landmark.label
        ? `label=${event.target.landmark.label}`
        : undefined,
      event.target.landmark.heading
        ? `heading=${event.target.landmark.heading}`
        : undefined,
    ].filter(Boolean);
    if (landmark.length)
      pushField(domLines, "Landmark/section", landmark.join(" "));
    pushField(domLines, "Landmark selector", event.target.landmark.selector);
  }
  if (event.target?.form) {
    pushField(domLines, "Form selector", event.target.form.selector);
    pushField(domLines, "Form action", event.target.form.action);
    pushField(domLines, "Form method", event.target.form.method);
    pushField(domLines, "Base form action", event.target.form.formAction);
    pushField(domLines, "Base form method", event.target.form.formMethod);
    if (event.target.form.submitter) {
      pushField(
        domLines,
        "Submitter selector",
        event.target.form.submitter.selector,
      );
      pushField(
        domLines,
        "Submitter action",
        event.target.form.submitter.action,
      );
      pushField(
        domLines,
        "Submitter method",
        event.target.form.submitter.method,
      );
    }
    if (event.target.form.fields?.length)
      domLines.push(`- Form fields: ${event.target.form.fields.join(" | ")}`);
    if (event.target.form.submitText?.length)
      domLines.push(
        `- Form submit buttons: ${event.target.form.submitText.join(" | ")}`,
      );
  }
  if (domLines.length) {
    lines.push("", "DOM context:", ...domLines);
  }

  const cheersLines: string[] = [];
  if (!rustSource) pushField(cheersLines, "Source hint", sourceHint);
  pushField(cheersLines, "Generated/nearest id", event.cheers?.generatedId);
  if (event.cheers?.datastar && Object.keys(event.cheers.datastar).length) {
    cheersLines.push(`- Datastar/data attrs: ${json(event.cheers.datastar)}`);
  }
  if (event.cheers?.datastarContext?.length) {
    cheersLines.push(
      `- Datastar ancestry: ${json(event.cheers.datastarContext)}`,
    );
  }
  if (cheersLines.length) {
    lines.push("", "Cheers/Datastar hints:", ...cheersLines);
  }

  if (rustSource) {
    lines.push(
      "",
      "Rust source context:",
      `- Location: ${rustSource.location}`,
      ...fenced("Focused element source excerpt", rustSource.snippet, "rust"),
    );
  }

  lines.push("", ...fenced("Target DOM excerpt", event.target?.htmlExcerpt));
  lines.push(
    "",
    "Treat the browser location as a hint, not authority. Inspect the Cheers code before editing. Use the normal skill-selection rules: load cheers for Cheers code work, and load cheers-design only if this requires UI/UX design judgment.",
  );

  return lines
    .filter((line, index, array) => !(line === "" && array[index - 1] === ""))
    .join("\n");
}

function normalizeEventPayload(
  payload: unknown,
  expectedToken: string,
): BrowserIterateEvent {
  if (!payload || typeof payload !== "object") {
    throw new Error("request body must be a JSON object");
  }
  const event = payload as Partial<BrowserIterateEvent>;
  if (event.version !== PROTOCOL_VERSION) {
    throw new Error(`unsupported protocol version: ${String(event.version)}`);
  }
  if (event.token !== expectedToken) {
    throw new Error("invalid token");
  }
  if (typeof event.prompt !== "string" || !event.prompt.trim()) {
    throw new Error("prompt is required");
  }
  return event as BrowserIterateEvent;
}

function sendJson(
  response: ServerResponse,
  status: number,
  body: unknown,
): void {
  response.writeHead(status, {
    "access-control-allow-origin": "*",
    "access-control-allow-methods": "GET, POST, OPTIONS",
    "access-control-allow-headers": "content-type",
    "cache-control": "no-store",
    "content-type": "application/json; charset=utf-8",
  });
  response.end(JSON.stringify(body));
}

function notFound(response: ServerResponse): void {
  sendJson(response, 404, { ok: false, error: "not_found" });
}

async function readBody(request: IncomingMessage): Promise<string> {
  let size = 0;
  const chunks: Buffer[] = [];
  for await (const chunk of request) {
    const buffer = Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk);
    size += buffer.byteLength;
    if (size > MAX_EVENT_BYTES) throw new Error("request body is too large");
    chunks.push(buffer);
  }
  return Buffer.concat(chunks).toString("utf8");
}

function requestUrl(request: IncomingMessage, origin: string): URL {
  return new URL(request.url || "/", origin);
}

function serveClientScript(
  response: ServerResponse,
  adapter: ActiveAdapter,
  request: IncomingMessage,
): void {
  const url = requestUrl(request, adapter.origin);
  if (url.searchParams.get("token") !== adapter.token) {
    sendJson(response, 403, { ok: false, error: "invalid_token" });
    return;
  }

  response.writeHead(200, {
    "access-control-allow-origin": "*",
    "cache-control": "no-store",
    "content-type": "text/javascript; charset=utf-8",
  });
  response.end(buildClientScript(adapter.origin, adapter.token));
}

async function handleEvent(
  response: ServerResponse,
  adapter: ActiveAdapter,
  request: IncomingMessage,
  pi: ExtensionAPI,
): Promise<void> {
  let payload: unknown;
  try {
    payload = JSON.parse(await readBody(request));
  } catch (error) {
    sendJson(response, 400, {
      ok: false,
      error: error instanceof Error ? error.message : String(error),
    });
    return;
  }

  let event: BrowserIterateEvent;
  try {
    event = normalizeEventPayload(payload, adapter.token);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    sendJson(response, message === "invalid token" ? 403 : 400, {
      ok: false,
      error: message,
    });
    return;
  }

  adapter.events += 1;
  const sourceHint = verifyRustSourceHint(
    event.cheers?.source,
    adapter.sourceHintSecret,
  );
  const rustSource = sourceHint
    ? await readRustSourceContext(sourceHint, adapter.projectRoot)
    : undefined;
  const message = formatAgentMessage(event, rustSource, sourceHint);
  pi.appendEntry("cheers-iterate-event", {
    at: Date.now(),
    url: event.page?.url,
    selector: event.target?.selector,
    source: sourceHint,
    rustSource: rustSource?.location,
    prompt: event.prompt,
  });
  pi.sendUserMessage(message, { deliverAs: "followUp" });
  sendJson(response, 200, {
    ok: true,
    result: { queued: true, events: adapter.events },
  });
}

async function writeMetadata(adapter: ActiveAdapter): Promise<void> {
  const metadata = {
    version: PROTOCOL_VERSION,
    origin: adapter.origin,
    token: adapter.token,
    sourceHintSecret: adapter.sourceHintSecret,
    projectRoot: normalizeProjectPath(adapter.projectRoot),
    pid: process.pid,
    startedAt: adapter.startedAt,
    command: "/cheers:iterate",
  };
  const metadataDir = dirname(adapter.metadataPath);
  await fs.mkdir(metadataDir, { recursive: true, mode: 0o700 });
  if (!adapter.metadataPathFromOverride) await fs.chmod(metadataDir, 0o700);
  try {
    await fs.chmod(adapter.metadataPath, 0o600);
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code !== "ENOENT") throw error;
  }
  await fs.writeFile(
    adapter.metadataPath,
    `${JSON.stringify(metadata, null, 2)}\n`,
    { encoding: "utf8", mode: 0o600 },
  );
  await fs.chmod(adapter.metadataPath, 0o600);
}

async function removeMetadata(adapter: ActiveAdapter): Promise<void> {
  try {
    const contents = await fs.readFile(adapter.metadataPath, "utf8");
    const metadata = JSON.parse(contents) as { token?: string };
    if (metadata.token !== adapter.token) return;
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === "ENOENT") return;
  }
  await fs.rm(adapter.metadataPath, { force: true });
}

async function stopActive(): Promise<void> {
  const adapter = active;
  if (!adapter) return;
  active = undefined;
  await new Promise<void>((resolve) => adapter.server.close(() => resolve()));
  await removeMetadata(adapter);
}

async function startAdapter(
  cwd: string,
  pi: ExtensionAPI,
): Promise<ActiveAdapter> {
  await stopActive();

  const token = randomBytes(24).toString("base64url");
  const sourceHintSecret = randomBytes(32).toString("base64url");
  const projectRoot = await canonicalProjectRoot(cwd);
  const server = createServer((request, response) => {
    void (async () => {
      if (request.method === "OPTIONS") {
        sendJson(response, 204, {});
        return;
      }

      const adapter = active;
      if (!adapter) {
        sendJson(response, 503, { ok: false, error: "adapter_not_started" });
        return;
      }

      const url = requestUrl(request, adapter.origin);
      if (request.method === "GET" && url.pathname === "/client.js") {
        serveClientScript(response, adapter, request);
        return;
      }
      if (request.method === "POST" && url.pathname === "/event") {
        await handleEvent(response, adapter, request, pi);
        return;
      }
      notFound(response);
    })().catch((error) => {
      sendJson(response, 500, {
        ok: false,
        error: error instanceof Error ? error.message : String(error),
      });
    });
  });

  await new Promise<void>((resolve, reject) => {
    server.once("error", reject);
    server.listen(0, HOST, () => {
      server.off("error", reject);
      resolve();
    });
  });

  const address = server.address();
  if (!address || typeof address === "string") {
    server.close();
    throw new Error("could not determine Cheers iterate adapter port");
  }

  const metadataPath = resolveMetadataPath(token);
  const adapter: ActiveAdapter = {
    server,
    projectRoot,
    metadataPath: metadataPath.path,
    metadataPathFromOverride: metadataPath.fromOverride,
    origin: `http://${HOST}:${address.port}`,
    token,
    sourceHintSecret,
    startedAt: new Date().toISOString(),
    events: 0,
  };
  active = adapter;
  try {
    await writeMetadata(adapter);
  } catch (error) {
    active = undefined;
    await new Promise<void>((resolve) => server.close(() => resolve()));
    throw error;
  }
  return adapter;
}

function statusLines(): string[] {
  return ["Cheers iterate is awaiting Alt-clicks in the browser."];
}

async function handleCommand(
  args: string,
  ctx: CommandContext,
  pi: ExtensionAPI,
): Promise<void> {
  const command = args.trim();
  if (command === "stop" || command === "off") {
    await stopActive();
    ctx.ui.setStatus("cheers-iterate", undefined);
    ctx.ui.setWidget("cheers-iterate", undefined);
    ctx.ui.notify("Cheers iterate stopped", "info");
    return;
  }

  if (command === "status") {
    if (!active) {
      ctx.ui.notify("Cheers iterate is not running", "warning");
      return;
    }
    ctx.ui.setWidget("cheers-iterate", statusLines());
    ctx.ui.notify(`Cheers iterate running at ${active.origin}`, "info");
    return;
  }

  if (command) {
    ctx.ui.notify("Usage: /cheers:iterate [status|stop]", "warning");
    return;
  }

  const adapter = await startAdapter(ctx.cwd, pi);
  ctx.ui.setWidget("cheers-iterate", statusLines());
  ctx.ui.notify(`Cheers iterate started at ${adapter.origin}`, "info");
}

export default function cheersExtension(pi: ExtensionAPI) {
  pi.registerCommand("cheers:iterate", {
    description:
      "Start a localhost browser-click bridge for Cheers UI iteration",
    handler: async (args, ctx) =>
      runCommandExclusive(() => handleCommand(args, ctx, pi)),
  });

  pi.on("session_shutdown", async () => {
    await runCommandExclusive(stopActive);
  });
}

const CLIENT_SCRIPT_TEMPLATE = String.raw`(() => {
  const ADAPTER_ORIGIN = __CHEERS_ITERATE_ADAPTER_ORIGIN__;
  const TOKEN = __CHEERS_ITERATE_TOKEN__;
  const VERSION = 1;
  const STYLE_ID = "cheers-iterate-style";
  const POPUP_ID = "cheers-iterate-popup";
  const SELECTED_ATTR = "data-cheers-iterate-selected";

  function truncate(value, max) {
    value = String(value || "");
    return value.length > max ? value.slice(0, max - 1) + "…" : value;
  }

  function addStyle() {
    if (document.getElementById(STYLE_ID)) return;
    const style = document.createElement("style");
    style.id = STYLE_ID;
    style.textContent =
      "[data-cheers-iterate-selected]{outline:2px solid #38bdf8!important;outline-offset:2px!important}" +
      "#" + POPUP_ID + "{position:fixed;z-index:2147483647;width:min(420px,calc(100vw - 24px));box-sizing:border-box;padding:12px;border:1px solid #334155;border-radius:12px;background:#0f172a;color:#e2e8f0;box-shadow:0 18px 48px #0008;font:12px/1.4 system-ui,-apple-system,BlinkMacSystemFont,Segoe UI,sans-serif}" +
      "#" + POPUP_ID + " textarea{box-sizing:border-box;width:100%;min-height:104px;margin:8px 0;padding:8px;border-radius:8px;border:1px solid #475569;background:#020617;color:#f8fafc;font:12px/1.4 system-ui,-apple-system,BlinkMacSystemFont,Segoe UI,sans-serif}" +
      "#" + POPUP_ID + " button{margin-right:6px;border:0;border-radius:999px;padding:6px 10px;background:#93c5fd;color:#0f172a;font-weight:700;cursor:pointer}" +
      "#" + POPUP_ID + " button.secondary{background:#cbd5e1}" +
      "#" + POPUP_ID + " .summary{margin-top:8px;padding:8px;border:1px solid #1e293b;border-radius:10px;background:#020617;color:#cbd5e1}" +
      "#" + POPUP_ID + " .row{display:grid;grid-template-columns:84px 1fr;gap:6px;margin:3px 0}" +
      "#" + POPUP_ID + " .label{color:#93c5fd;font-weight:700}" +
      "#" + POPUP_ID + " .muted{color:#94a3b8}" +
      "#" + POPUP_ID + " .status{margin-top:6px;color:#c4b5fd;white-space:pre-wrap}";
    document.head.appendChild(style);
  }

  function removePopup() {
    const old = document.getElementById(POPUP_ID);
    if (old) old.remove();
  }

  function cssEscape(value) {
    if (window.CSS && typeof window.CSS.escape === "function") return window.CSS.escape(value);
    return String(value).replace(/[^a-zA-Z0-9_-]/g, "\\$&");
  }

  function selectorFor(element) {
    if (!element || element.nodeType !== 1) return undefined;
    if (element.id) return "#" + cssEscape(element.id);
    const parts = [];
    let node = element;
    while (node && node.nodeType === 1 && node !== document.body) {
      let part = node.localName;
      if (!part) break;
      const parent = node.parentElement;
      if (!parent) break;
      const siblings = Array.prototype.filter.call(parent.children, function(child) {
        return child.localName === node.localName;
      });
      if (siblings.length > 1) part += ":nth-of-type(" + (siblings.indexOf(node) + 1) + ")";
      parts.unshift(part);
      node = parent;
      if (parts.length >= 6) break;
    }
    return parts.length ? parts.join(" > ") : undefined;
  }

  function attrs(element, predicate, limit) {
    const result = {};
    if (!element || !element.attributes) return result;
    for (const attr of Array.from(element.attributes)) {
      if (predicate(attr.name, attr.value)) {
        result[attr.name] = truncate(attr.value, 500);
        if (Object.keys(result).length >= limit) break;
      }
    }
    return result;
  }

  function attrsInAncestry(element, predicate, limit) {
    const result = {};
    let node = element;
    let depth = 0;
    while (node && node.nodeType === 1 && depth < 6) {
      for (const attr of Array.from(node.attributes || [])) {
        if (predicate(attr.name, attr.value) && result[attr.name] === undefined) {
          result[attr.name] = truncate(attr.value, 500);
          if (Object.keys(result).length >= limit) return result;
        }
      }
      node = node.parentElement;
      depth += 1;
    }
    return result;
  }

  function nearestAttr(element, name) {
    const node = element && element.closest && element.closest("[" + name + "]");
    return node ? node.getAttribute(name) || undefined : undefined;
  }

  function nearestId(element) {
    const node = element && element.closest && element.closest("[id]");
    return node ? node.id || undefined : undefined;
  }

  function pickTarget(element) {
    return element && element.closest && (
      element.closest("button,a,input,select,textarea,label,form,[role],[id]") ||
      element.closest("[data-cheers-source]") ||
      element
    );
  }

  function textOf(element) {
    return truncate((element && element.innerText) || (element && element.textContent) || "", 500).trim();
  }

  function attrValue(element, name) {
    const value = element && element.getAttribute && element.getAttribute(name);
    return value ? truncate(value, 500) : undefined;
  }

  function uniqueNonEmpty(values, limit) {
    const seen = new Set();
    const result = [];
    for (const value of values) {
      const text = truncate(String(value || "").replace(/\s+/g, " ").trim(), 300);
      if (!text || seen.has(text)) continue;
      seen.add(text);
      result.push(text);
      if (result.length >= limit) break;
    }
    return result;
  }

  function isDatastarAttr(name) {
    return /^data-(on|bind|signal|signals|indicator|show|class|attr|text|computed|persist|replace-url|scroll-into-view|view-transition)(-|$)/.test(name);
  }

  function ariaAttributes(element) {
    return attrs(element, function(name) {
      return name.indexOf("aria-") === 0;
    }, 20);
  }

  function labelTexts(element) {
    if (!element || element.nodeType !== 1) return [];
    const values = [];
    const ariaLabel = attrValue(element, "aria-label");
    if (ariaLabel) values.push(ariaLabel);

    const labelledBy = attrValue(element, "aria-labelledby");
    if (labelledBy) {
      for (const id of labelledBy.split(/\s+/)) {
        const labelled = id && document.getElementById(id);
        if (labelled) values.push(textOf(labelled));
      }
    }

    if (element.labels) {
      for (const label of Array.from(element.labels)) values.push(textOf(label));
    }
    if (element.id) {
      for (const label of Array.from(document.querySelectorAll('label[for="' + cssEscape(element.id) + '"]'))) values.push(textOf(label));
    }
    const wrappingLabel = element.closest && element.closest("label");
    if (wrappingLabel) values.push(textOf(wrappingLabel));

    return uniqueNonEmpty(values, 8);
  }

  function inputContext(element) {
    const control = element && element.matches && (element.matches("input,select,textarea,button") ? element : element.closest("input,select,textarea,button"));
    if (!control) return undefined;
    const result = {
      type: attrValue(control, "type") || control.localName,
      name: attrValue(control, "name"),
      placeholder: attrValue(control, "placeholder"),
      checked: typeof control.checked === "boolean" ? control.checked : undefined,
      disabled: control.disabled ? true : undefined,
      required: control.required ? true : undefined
    };
    Object.keys(result).forEach(function(key) {
      if (result[key] === undefined || result[key] === "") delete result[key];
    });
    return Object.keys(result).length ? result : undefined;
  }

  function submitterFor(element) {
    const control = element && element.closest && element.closest("button,input");
    if (!control) return undefined;
    if (control.localName === "button") {
      const type = (control.type || attrValue(control, "type") || "submit").toLowerCase();
      return !type || type === "submit" ? control : undefined;
    }
    const type = (control.type || attrValue(control, "type") || "text").toLowerCase();
    return type === "submit" || type === "image" ? control : undefined;
  }

  function submitterAction(submitter) {
    if (!submitter || !submitter.hasAttribute || !submitter.hasAttribute("formaction")) return undefined;
    return attrValue(submitter, "formaction") || submitter.formAction || window.location.href;
  }

  function submitterMethod(submitter) {
    if (!submitter || !submitter.hasAttribute || !submitter.hasAttribute("formmethod")) return undefined;
    return (submitter.formMethod || attrValue(submitter, "formmethod") || "get").toLowerCase();
  }

  const HEADING_SELECTOR = "h1,h2,h3,h4,h5,h6";

  function firstHeadingWithin(element) {
    if (!element || element.nodeType !== 1) return undefined;
    if (element.matches && element.matches(HEADING_SELECTOR)) return element;
    const headings = element.querySelectorAll ? element.querySelectorAll(HEADING_SELECTOR) : [];
    return headings.length ? headings[0] : undefined;
  }

  function landmarkContext(element) {
    const landmark = element && element.closest && element.closest("main,section,article,aside,nav,header,footer,dialog,[role=main],[role=region],[role=form],[role=navigation],[role=banner],[role=contentinfo],[role=complementary],[role=dialog]");
    if (!landmark) return undefined;
    const heading = firstHeadingWithin(landmark);
    return {
      tag: landmark.localName,
      role: attrValue(landmark, "role"),
      label: attrValue(landmark, "aria-label") || attrValue(landmark, "aria-labelledby"),
      selector: selectorFor(landmark),
      heading: heading ? textOf(heading) : undefined
    };
  }

  function formContext(form, submitter) {
    if (!form) return undefined;
    const fields = Array.from(form.elements || []).map(function(field) {
      const labels = labelTexts(field);
      return uniqueNonEmpty([
        field.localName,
        attrValue(field, "type"),
        attrValue(field, "name"),
        field.id ? "#" + field.id : undefined,
        labels.length ? "label=" + labels[0] : undefined
      ], 6).join(" ");
    }).filter(Boolean).slice(0, 20);
    const submitText = Array.from(form.querySelectorAll("button,input[type=submit],input[type=button]")).map(function(button) {
      return textOf(button) || attrValue(button, "value") || attrValue(button, "aria-label");
    });
    const formAction = attrValue(form, "action") || form.action || undefined;
    const formMethod = (form.method || attrValue(form, "method") || "get").toLowerCase();
    const overrideAction = submitterAction(submitter);
    const overrideMethod = submitterMethod(submitter);
    const result = {
      action: overrideAction || formAction,
      method: overrideMethod || formMethod,
      selector: selectorFor(form),
      fields: uniqueNonEmpty(fields, 20),
      submitText: uniqueNonEmpty(submitText, 8)
    };
    if (overrideAction || overrideMethod) {
      result.submitter = {
        selector: selectorFor(submitter),
        action: overrideAction,
        method: overrideMethod
      };
      Object.keys(result.submitter).forEach(function(key) {
        if (result.submitter[key] === undefined || result.submitter[key] === "") delete result.submitter[key];
      });
      result.formAction = formAction;
      result.formMethod = formMethod;
    }
    if (!result.fields.length) delete result.fields;
    if (!result.submitText.length) delete result.submitText;
    return result;
  }

  function datastarContext(element) {
    const entries = [];
    let node = element;
    let depth = 0;
    while (node && node.nodeType === 1 && depth < 8) {
      const attributes = attrs(node, isDatastarAttr, 20);
      if (Object.keys(attributes).length) {
        entries.push({ tag: node.localName, selector: selectorFor(node), attributes: attributes });
        if (entries.length >= 8) break;
      }
      node = node.parentElement;
      depth += 1;
    }
    return entries;
  }

  function sanitizedOuterHtml(element) {
    const clone = element.cloneNode(true);
    if (clone.nodeType === 1) clone.removeAttribute("data-cheers-source");
    if (clone.querySelectorAll) {
      clone.querySelectorAll("[data-cheers-source]").forEach(function(node) {
        node.removeAttribute("data-cheers-source");
      });
    }
    return clone.outerHTML;
  }

  function collectContext(target) {
    const submitter = submitterFor(target);
    const form = (submitter && submitter.form) || (target.closest && target.closest("form"));
    const datastarAttrs = attrsInAncestry(target, isDatastarAttr, 30);
    const datastarEntries = datastarContext(target);
    const labels = labelTexts(target);
    const aria = ariaAttributes(target);
    const input = inputContext(target);
    const landmark = landmarkContext(target);
    const formDetails = formContext(form, submitter);
    return {
      page: {
        url: window.location.href,
        title: document.title
      },
      target: {
        tag: target.localName,
        id: target.id || undefined,
        classes: Array.from(target.classList || []).slice(0, 20),
        role: target.getAttribute("role") || undefined,
        href: target.href || attrValue(target, "href"),
        text: textOf(target),
        accessibleName: target.getAttribute("aria-label") || target.getAttribute("title") || labels[0] || textOf(target),
        selector: selectorFor(target),
        input: input,
        labels: labels.length ? labels : undefined,
        aria: Object.keys(aria).length ? aria : undefined,
        landmark: landmark,
        form: formDetails,
        htmlExcerpt: truncate(sanitizedOuterHtml(target), 2000)
      },
      cheers: {
        source: nearestAttr(target, "data-cheers-source"),
        generatedId: target.id || nearestId(target),
        datastar: Object.keys(datastarAttrs).length ? datastarAttrs : undefined,
        datastarContext: datastarEntries.length ? datastarEntries : undefined
      }
    };
  }

  function eventPayload(context, prompt) {
    return Object.assign({ version: VERSION, token: TOKEN, prompt: prompt }, context);
  }

  async function submit(context, textarea, status) {
    const prompt = textarea.value.trim();
    if (!prompt) {
      status.textContent = "Describe what should change first.";
      textarea.focus();
      return;
    }
    status.textContent = "Sending to Pi…";
    const response = await fetch(ADAPTER_ORIGIN + "/event", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(eventPayload(context, prompt))
    });
    const body = await response.json();
    if (!response.ok || !body.ok) {
      throw new Error(body.error || ("HTTP " + response.status));
    }
    status.textContent = "Sent to Pi. You can close this popup.";
  }

  function displayTarget(context) {
    const target = context.target || {};
    return truncate([target.selector || target.tag, target.text].filter(Boolean).join(" · "), 180);
  }

  function appendSummaryRow(summary, label, value) {
    if (!value) return;
    const row = document.createElement("div");
    row.className = "row";
    const labelNode = document.createElement("div");
    labelNode.className = "label";
    labelNode.textContent = label;
    const valueNode = document.createElement("div");
    valueNode.textContent = truncate(value, 220);
    row.append(labelNode, valueNode);
    summary.appendChild(row);
  }

  function createSummary(context) {
    const summary = document.createElement("div");
    summary.className = "summary";
    appendSummaryRow(summary, "Target", displayTarget(context));
    appendSummaryRow(summary, "Source", context.cheers && context.cheers.source);
    appendSummaryRow(summary, "ID", context.cheers && context.cheers.generatedId);
    appendSummaryRow(summary, "Labels", context.target && context.target.labels && context.target.labels.join(" | "));
    appendSummaryRow(summary, "Form", context.target && context.target.form && [context.target.form.method, context.target.form.action || context.target.form.selector].filter(Boolean).join(" "));
    if (context.cheers && context.cheers.datastarContext && context.cheers.datastarContext.length) {
      appendSummaryRow(summary, "Datastar", context.cheers.datastarContext.map(function(entry) {
        return Object.keys(entry.attributes || {}).join(", ");
      }).filter(Boolean).join(" | "));
    }
    return summary;
  }

  function openPopup(target, event) {
    addStyle();
    removePopup();
    document.querySelectorAll("[" + SELECTED_ATTR + "]").forEach(function(node) {
      node.removeAttribute(SELECTED_ATTR);
    });
    const context = collectContext(target);
    target.setAttribute(SELECTED_ATTR, "");

    const panel = document.createElement("div");
    panel.id = POPUP_ID;
    panel.style.left = Math.max(12, Math.min(event.clientX + 12, window.innerWidth - 432)) + "px";
    panel.style.top = Math.max(12, Math.min(event.clientY + 12, window.innerHeight - 340)) + "px";

    const summary = createSummary(context);
    const textarea = document.createElement("textarea");
    textarea.placeholder = "What should change here?";
    const send = document.createElement("button");
    send.textContent = "Send to Pi";
    const cancel = document.createElement("button");
    cancel.textContent = "Cancel";
    cancel.className = "secondary";
    const status = document.createElement("div");
    status.className = "status";

    send.addEventListener("click", function() {
      submit(context, textarea, status).catch(function(error) {
        const message = error && error.message ? error.message : String(error);
        status.textContent = "Failed: " + message;
      });
    });
    cancel.addEventListener("click", function() {
      target.removeAttribute(SELECTED_ATTR);
      removePopup();
    });
    textarea.addEventListener("keydown", function(keyEvent) {
      if ((keyEvent.metaKey || keyEvent.ctrlKey) && keyEvent.key === "Enter") send.click();
      if (keyEvent.key === "Escape") cancel.click();
    });

    panel.append(summary, textarea, send, cancel, status);
    document.body.appendChild(panel);
    textarea.focus();
  }

  window.addEventListener("click", function(event) {
    if (!event.altKey || event.button !== 0) return;
    const rawTarget = event.target && event.target.closest && event.target.closest("*");
    if (!rawTarget || rawTarget.closest("#" + POPUP_ID)) return;
    const target = pickTarget(rawTarget);
    if (!target) return;
    event.preventDefault();
    event.stopPropagation();
    openPopup(target, event);
  }, true);
})();`;

function buildClientScript(origin: string, token: string): string {
  return CLIENT_SCRIPT_TEMPLATE.replace(
    "__CHEERS_ITERATE_ADAPTER_ORIGIN__",
    JSON.stringify(origin),
  ).replace("__CHEERS_ITERATE_TOKEN__", JSON.stringify(token));
}
