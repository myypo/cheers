export function attrSelector(name: string, value: string): string {
  return (
    "[" +
    name +
    '="' +
    String(value || "")
      .replace(/\\/g, "\\\\")
      .replace(/"/g, '\\"') +
    '"]'
  );
}

function ssrStream(key: string): void {
  const template = document.querySelector(attrSelector("data-ssr", key + "-t"));
  const target = document.querySelector(attrSelector("data-ssr", key));
  const script = document.querySelector(attrSelector("data-ssr", key + "-s"));

  if (!(template instanceof HTMLTemplateElement) || !target) {
    console.warn("Cheers async stream target missing", key);
    return;
  }

  target.replaceWith(template.content);
  template.remove();
  if (script) script.remove();
}

declare global {
  interface Window {
    __ssrStream: typeof ssrStream;
  }
}

window.__ssrStream = ssrStream;
