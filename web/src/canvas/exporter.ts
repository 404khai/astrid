/** Exports the drawing as you see it: a standalone SVG with every style
 * resolved and inlined (so it opens anywhere), or a 2x PNG made from it. */

const props = [
  "fill", "fill-opacity", "stroke", "stroke-width", "stroke-dasharray", "stroke-dashoffset", "stroke-opacity",
  "stroke-linecap", "stroke-linejoin", "opacity", "font-family", "font-size", "font-weight", "letter-spacing",
  "text-anchor", "filter", "display",
];

export function drawingSvg(svg: SVGSVGElement, background: string, width: number, height: number): string {
  const copy = svg.cloneNode(true) as SVGSVGElement;
  const from = [svg, ...svg.querySelectorAll("*")];
  const to = [copy, ...copy.querySelectorAll("*")];
  from.forEach((el, i) => {
    const css = getComputedStyle(el);
    const out = to[i] as SVGElement;
    out.removeAttribute("class");
    out.setAttribute("style", props.map((p) => `${p}:${css.getPropertyValue(p)}`).join(";") + ";animation:none");
  });
  // The pan/zoom lives in a CSS transform; SVG viewers need an attribute.
  const view = svg.querySelector<SVGGElement>(".cv-view");
  const viewCopy = to[from.indexOf(view!)] as SVGGElement | undefined;
  if (view && viewCopy) {
    const m = new DOMMatrix(getComputedStyle(view).transform);
    viewCopy.setAttribute("transform", `matrix(${m.a} ${m.b} ${m.c} ${m.d} ${m.e} ${m.f})`);
  }
  copy.setAttribute("xmlns", "http://www.w3.org/2000/svg");
  copy.setAttribute("width", String(width));
  copy.setAttribute("height", String(height));
  const sheet = document.createElementNS("http://www.w3.org/2000/svg", "rect");
  sheet.setAttribute("width", "100%");
  sheet.setAttribute("height", "100%");
  sheet.setAttribute("fill", background);
  copy.insertBefore(sheet, copy.firstChild);
  return new XMLSerializer().serializeToString(copy);
}

export async function drawingPng(svgText: string, width: number, height: number, scale = 2): Promise<string> {
  const image = new Image();
  image.src = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svgText)}`;
  await image.decode();
  const canvas = document.createElement("canvas");
  canvas.width = width * scale;
  canvas.height = height * scale;
  const context = canvas.getContext("2d")!;
  context.scale(scale, scale);
  context.drawImage(image, 0, 0, width, height);
  return canvas.toDataURL("image/png").split(",")[1];
}

/** A file-safe name from the drawing's title. */
export const fileName = (title: string | null) =>
  (title ?? "Astrid drawing").replace(/[^\p{L}\p{N} _-]+/gu, "").trim().slice(0, 60) || "Astrid drawing";
