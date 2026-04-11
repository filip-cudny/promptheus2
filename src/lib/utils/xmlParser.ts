export interface XmlNode {
  tag: string;
  attributes: Record<string, string>;
  children: XmlNode[];
  text: string | null;
}

export function parseXml(xml: string): XmlNode | null {
  try {
    const parser = new DOMParser();
    const doc = parser.parseFromString(xml, "text/xml");
    const error = doc.querySelector("parsererror");
    if (error) return null;
    return domToXmlNode(doc.documentElement!);
  } catch {
    return null;
  }
}

function domToXmlNode(el: Element): XmlNode {
  const attributes: Record<string, string> = {};
  for (const attr of el.attributes) {
    attributes[attr.name] = attr.value;
  }

  const children: XmlNode[] = [];
  let text: string | null = null;
  const textParts: string[] = [];

  for (const child of el.childNodes) {
    if (child.nodeType === Node.ELEMENT_NODE) {
      children.push(domToXmlNode(child as Element));
    } else if (child.nodeType === Node.TEXT_NODE || child.nodeType === Node.CDATA_SECTION_NODE) {
      const t = child.textContent?.trim();
      if (t) textParts.push(t);
    }
  }

  if (textParts.length > 0 && children.length === 0) {
    text = textParts.join("\n");
  }

  return { tag: el.tagName, attributes, children, text };
}

export function isXmlLike(text: string): boolean {
  return /^\s*<[a-zA-Z_][\w.-]*[\s>]/.test(text);
}
