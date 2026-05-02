import { attrSelector } from "./ssr-stream";

function ssrTarget(root: ParentNode, key: string): Element | null {
  const exact = root.querySelector(attrSelector("data-ssr", key));
  if (exact) return exact;

  // During Subsecond hot patches, old and new generated async-stream functions can
  // briefly coexist. If an edit moved an `@async` block, the fallback anchor can have
  // the new source key while the streamed template still has the old key. Prefer the
  // single unresolved async root so the page does not remain stuck on the fallback.
  const roots = Array.from(
    root.querySelectorAll("[data-cheers-async-root][data-ssr]"),
  );
  if (roots.length === 1) return roots[0];

  return null;
}

function applySsrTemplate(
  root: ParentNode,
  key: string,
  template: HTMLTemplateElement,
  removeIfMissing = false,
): void {
  const target = ssrTarget(root, key);
  const script = root.querySelector(attrSelector("data-ssr", key + "-s"));

  if (!target) {
    console.warn("Cheers async stream target missing", key);
    if (removeIfMissing) {
      template.remove();
      if (script) script.remove();
    }
    return;
  }

  if (target.hasAttribute("data-cheers-async-root")) {
    target.replaceChildren(template.content.cloneNode(true));
    target.removeAttribute("data-ssr");
  } else {
    target.replaceWith(template.content.cloneNode(true));
  }
  template.remove();
  if (script) script.remove();
}

function ssrStream(key: string): void {
  const template = document.querySelector(attrSelector("data-ssr", key + "-t"));
  if (!(template instanceof HTMLTemplateElement)) {
    console.warn("Cheers async stream target missing", key);
    return;
  }

  applySsrTemplate(document, key, template);
}

function settleSsrStreams(root: ParentNode): void {
  for (const template of Array.from(
    root.querySelectorAll("template[data-ssr$='-t']"),
  )) {
    if (!(template instanceof HTMLTemplateElement)) continue;
    const marker = template.getAttribute("data-ssr") || "";
    applySsrTemplate(root, marker.slice(0, -2), template, true);
  }
}

declare global {
  interface Window {
    __cheersSsrAttrSelector: typeof attrSelector;
    __cheersSettleSsrStreams: typeof settleSsrStreams;
    __ssrStream: typeof ssrStream;
  }
}

window.__cheersSsrAttrSelector = attrSelector;
window.__cheersSettleSsrStreams = settleSsrStreams;
window.__ssrStream = ssrStream;
