import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";
import { randomBytes } from "node:crypto";
import { createServer, type IncomingMessage, type Server, type ServerResponse } from "node:http";
import { promises as fs } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";

const PROTOCOL_VERSION = 1;
const HOST = "127.0.0.1";
const MAX_EVENT_BYTES = 128 * 1024;

interface ActiveAdapter {
	server: Server;
	projectRoot: string;
	metadataPath: string;
	origin: string;
	token: string;
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
		viewport?: { width?: number; height?: number };
	};
	click?: { clientX?: number; clientY?: number };
	target?: {
		tag?: string;
		id?: string;
		classes?: string[];
		role?: string;
		href?: string;
		text?: string;
		accessibleName?: string;
		selector?: string;
		rect?: { x?: number; y?: number; width?: number; height?: number };
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
		nearestHeading?: { tag?: string; text?: string; selector?: string };
		landmark?: { tag?: string; role?: string; label?: string; selector?: string; heading?: string };
		form?: {
			action?: string;
			method?: string;
			selector?: string;
			fields?: string[];
			submitText?: string[];
		};
		nearbyText?: string;
		htmlExcerpt?: string;
		ancestorExcerpt?: string;
	};
	cheers?: {
		component?: string;
		source?: string;
		generatedId?: string;
		formAction?: string;
		formMethod?: string;
		datastar?: Record<string, string>;
		datastarContext?: Array<{ tag?: string; selector?: string; attributes?: Record<string, string> }>;
	};
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

function resolveMetadataPath(token: string): string {
	const override = process.env.CHEERS_ITERATE_METADATA?.trim();
	if (override) return override;

	const safeName = `${process.pid}-${Date.now().toString(36)}-${token.slice(0, 12)}.json`;
	return join(cheersIterateMetadataDir(), safeName);
}

function oneLine(value: unknown): string | undefined {
	if (typeof value !== "string") return undefined;
	const trimmed = value.replace(/\s+/g, " ").trim();
	return trimmed || undefined;
}

function numberLine(value: unknown): string | undefined {
	return typeof value === "number" && Number.isFinite(value) ? String(Math.round(value)) : undefined;
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

function formatAgentMessage(event: BrowserIterateEvent): string {
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
	if (event.target?.classes?.length) lines.push(`- Classes: ${event.target.classes.join(" ")}`);
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
	if (event.target?.labels?.length) lines.push(`- Labels: ${event.target.labels.join(" | ")}`);
	if (event.target?.aria && Object.keys(event.target.aria).length) {
		lines.push(`- ARIA attrs: ${json(event.target.aria)}`);
	}

	if (event.target?.rect) {
		const { x, y, width, height } = event.target.rect;
		const parts = [
			numberLine(x) ? `x=${numberLine(x)}` : undefined,
			numberLine(y) ? `y=${numberLine(y)}` : undefined,
			numberLine(width) ? `w=${numberLine(width)}` : undefined,
			numberLine(height) ? `h=${numberLine(height)}` : undefined,
		].filter(Boolean);
		if (parts.length) lines.push(`- Rect: ${parts.join(" ")}`);
	}

	if (event.click) {
		const x = numberLine(event.click.clientX);
		const y = numberLine(event.click.clientY);
		if (x && y) lines.push(`- Click: x=${x} y=${y}`);
	}

	if (event.page?.viewport) {
		const width = numberLine(event.page.viewport.width);
		const height = numberLine(event.page.viewport.height);
		if (width && height) lines.push(`- Viewport: ${width}x${height}`);
	}

	const domLines: string[] = [];
	if (event.target?.nearestHeading) {
		const heading = [event.target.nearestHeading.tag, event.target.nearestHeading.text].filter(Boolean).join(" ");
		pushField(domLines, "Nearest heading", heading);
	}
	if (event.target?.landmark) {
		const landmark = [
			event.target.landmark.tag,
			event.target.landmark.role ? `role=${event.target.landmark.role}` : undefined,
			event.target.landmark.label ? `label=${event.target.landmark.label}` : undefined,
			event.target.landmark.heading ? `heading=${event.target.landmark.heading}` : undefined,
		].filter(Boolean);
		if (landmark.length) pushField(domLines, "Landmark/section", landmark.join(" "));
		pushField(domLines, "Landmark selector", event.target.landmark.selector);
	}
	if (event.target?.form) {
		pushField(domLines, "Form selector", event.target.form.selector);
		pushField(domLines, "Form action", event.target.form.action);
		pushField(domLines, "Form method", event.target.form.method);
		if (event.target.form.fields?.length) domLines.push(`- Form fields: ${event.target.form.fields.join(" | ")}`);
		if (event.target.form.submitText?.length) domLines.push(`- Form submit buttons: ${event.target.form.submitText.join(" | ")}`);
	}
	pushField(domLines, "Nearby text", event.target?.nearbyText);
	if (domLines.length) {
		lines.push("", "DOM context:", ...domLines);
	}

	const cheersLines: string[] = [];
	pushField(cheersLines, "Component", event.cheers?.component);
	pushField(cheersLines, "Source", event.cheers?.source);
	pushField(cheersLines, "Generated/nearest id", event.cheers?.generatedId);
	pushField(cheersLines, "Form action", event.cheers?.formAction);
	pushField(cheersLines, "Form method", event.cheers?.formMethod);
	if (event.cheers?.datastar && Object.keys(event.cheers.datastar).length) {
		cheersLines.push(`- Datastar/data attrs: ${json(event.cheers.datastar)}`);
	}
	if (event.cheers?.datastarContext?.length) {
		cheersLines.push(`- Datastar ancestry: ${json(event.cheers.datastarContext)}`);
	}
	if (cheersLines.length) {
		lines.push("", "Cheers/Datastar hints:", ...cheersLines);
	}

	lines.push("", ...fenced("Target DOM excerpt", event.target?.htmlExcerpt));
	lines.push("", ...fenced("Nearest useful ancestor excerpt", event.target?.ancestorExcerpt));
	lines.push(
		"",
		"Treat the browser location as a hint, not authority. Inspect the Cheers code before editing. Use the normal skill-selection rules: load cheers for Cheers code work, and load cheers-design only if this requires UI/UX design judgment.",
	);

	return lines.filter((line, index, array) => !(line === "" && array[index - 1] === "")).join("\n");
}

function normalizeEventPayload(payload: unknown, expectedToken: string): BrowserIterateEvent {
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

function sendJson(response: ServerResponse, status: number, body: unknown): void {
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

function serveClientScript(response: ServerResponse, adapter: ActiveAdapter, request: IncomingMessage): void {
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
		sendJson(response, message === "invalid token" ? 403 : 400, { ok: false, error: message });
		return;
	}

	adapter.events += 1;
	const message = formatAgentMessage(event);
	pi.appendEntry("cheers-iterate-event", {
		at: Date.now(),
		url: event.page?.url,
		selector: event.target?.selector,
		component: event.cheers?.component,
		source: event.cheers?.source,
		prompt: event.prompt,
	});
	pi.sendUserMessage(message, { deliverAs: "followUp" });
	sendJson(response, 200, { ok: true, result: { queued: true, events: adapter.events } });
}

async function writeMetadata(adapter: ActiveAdapter): Promise<void> {
	const metadata = {
		version: PROTOCOL_VERSION,
		origin: adapter.origin,
		token: adapter.token,
		projectRoot: normalizeProjectPath(adapter.projectRoot),
		pid: process.pid,
		startedAt: adapter.startedAt,
		command: "/cheers:iterate",
	};
	await fs.mkdir(dirname(adapter.metadataPath), { recursive: true });
	await fs.writeFile(adapter.metadataPath, `${JSON.stringify(metadata, null, 2)}\n`, "utf8");
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

async function startAdapter(cwd: string, pi: ExtensionAPI): Promise<ActiveAdapter> {
	await stopActive();

	const token = randomBytes(24).toString("base64url");
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

	const adapter: ActiveAdapter = {
		server,
		projectRoot,
		metadataPath: resolveMetadataPath(token),
		origin: `http://${HOST}:${address.port}`,
		token,
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

async function handleCommand(args: string, ctx: CommandContext, pi: ExtensionAPI): Promise<void> {
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
		description: "Start a localhost browser-click bridge for Cheers UI iteration",
		handler: async (args, ctx) => runCommandExclusive(() => handleCommand(args, ctx, pi)),
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

  function usefulAncestor(element) {
    return element && element.closest && element.closest("[data-cheers-component],[data-cheers-source],form,main,section,article,aside,nav,header,footer,dialog,[role]");
  }

  function pickTarget(element) {
    return element && element.closest && (element.closest("[data-cheers-source],[data-cheers-component],button,a,input,select,textarea,label,form,[role],[id]") || element);
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

  const HEADING_SELECTOR = "h1,h2,h3,h4,h5,h6";

  function lastHeadingWithin(element) {
    if (!element || element.nodeType !== 1) return undefined;
    if (element.matches && element.matches(HEADING_SELECTOR)) return element;
    const headings = element.querySelectorAll ? element.querySelectorAll(HEADING_SELECTOR) : [];
    return headings.length ? headings[headings.length - 1] : undefined;
  }

  function firstHeadingWithin(element) {
    if (!element || element.nodeType !== 1) return undefined;
    if (element.matches && element.matches(HEADING_SELECTOR)) return element;
    const headings = element.querySelectorAll ? element.querySelectorAll(HEADING_SELECTOR) : [];
    return headings.length ? headings[0] : undefined;
  }

  function headingSummary(heading) {
    return heading ? { tag: heading.localName, text: textOf(heading), selector: selectorFor(heading) } : undefined;
  }

  function nearestHeading(element) {
    let node = element;
    while (node && node.nodeType === 1 && node !== document.body) {
      let previous = node.previousElementSibling;
      while (previous) {
        const heading = lastHeadingWithin(previous);
        if (heading) return headingSummary(heading);
        previous = previous.previousElementSibling;
      }
      if (node.matches && node.matches(HEADING_SELECTOR)) return headingSummary(node);
      node = node.parentElement;
    }
    return headingSummary(firstHeadingWithin(document.body));
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

  function formContext(form) {
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
    const result = {
      action: attrValue(form, "action") || form.action || undefined,
      method: (attrValue(form, "method") || form.method || "get").toLowerCase(),
      selector: selectorFor(form),
      fields: uniqueNonEmpty(fields, 20),
      submitText: uniqueNonEmpty(submitText, 8)
    };
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

  function collectContext(target, event) {
    const rect = target.getBoundingClientRect();
    const form = target.closest && target.closest("form");
    const ancestor = usefulAncestor(target);
    const datastarAttrs = attrsInAncestry(target, isDatastarAttr, 30);
    const datastarEntries = datastarContext(target);
    const labels = labelTexts(target);
    const aria = ariaAttributes(target);
    const input = inputContext(target);
    const heading = nearestHeading(target);
    const landmark = landmarkContext(target);
    const formDetails = formContext(form);
    return {
      page: {
        url: window.location.href,
        title: document.title,
        viewport: { width: window.innerWidth, height: window.innerHeight }
      },
      click: { clientX: event.clientX, clientY: event.clientY },
      target: {
        tag: target.localName,
        id: target.id || undefined,
        classes: Array.from(target.classList || []).slice(0, 20),
        role: target.getAttribute("role") || undefined,
        href: target.href || attrValue(target, "href"),
        text: textOf(target),
        accessibleName: target.getAttribute("aria-label") || target.getAttribute("title") || labels[0] || textOf(target),
        selector: selectorFor(target),
        rect: { x: rect.x, y: rect.y, width: rect.width, height: rect.height },
        input: input,
        labels: labels.length ? labels : undefined,
        aria: Object.keys(aria).length ? aria : undefined,
        nearestHeading: heading,
        landmark: landmark,
        form: formDetails,
        nearbyText: ancestor ? truncate(textOf(ancestor), 1000) : undefined,
        htmlExcerpt: truncate(target.outerHTML, 4000),
        ancestorExcerpt: ancestor ? truncate(ancestor.outerHTML, 6000) : undefined
      },
      cheers: {
        component: nearestAttr(target, "data-cheers-component"),
        source: nearestAttr(target, "data-cheers-source"),
        generatedId: target.id || nearestId(target),
        formAction: target.getAttribute("formaction") || (form ? form.getAttribute("action") || undefined : undefined),
        formMethod: target.getAttribute("formmethod") || (form ? form.getAttribute("method") || "get" : undefined),
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
    const body = await response.json().catch(function() { return {}; });
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
    appendSummaryRow(summary, "Component", context.cheers && context.cheers.component);
    appendSummaryRow(summary, "Source", context.cheers && context.cheers.source);
    appendSummaryRow(summary, "ID", context.cheers && context.cheers.generatedId);
    appendSummaryRow(summary, "Heading", context.target && context.target.nearestHeading && context.target.nearestHeading.text);
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
    const context = collectContext(target, event);
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
	return CLIENT_SCRIPT_TEMPLATE
		.replace("__CHEERS_ITERATE_ADAPTER_ORIGIN__", JSON.stringify(origin))
		.replace("__CHEERS_ITERATE_TOKEN__", JSON.stringify(token));
}
