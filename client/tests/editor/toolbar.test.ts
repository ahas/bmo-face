import { describe, expect, test } from "bun:test";

import { parse } from "vue/compiler-sfc";

// Where the editor's controls stand and at what size (docs/face.md, 「The screens' shared style」): the
// top bar holds the screen's name and logging out alone, the actions stand in the toolbar over the
// canvas with leaving a part's inside at its left end, and every control in that toolbar is the
// library's small one at its own size, the theme setting no size of its own on an icon button.

(globalThis as { defineAppConfig?: unknown }).defineAppConfig = (config: unknown) => config;

// What of the compiler's tree the checks read: an element, its tag, children and props, and a prop's
// name, its plain value or its directive's argument.
interface TemplateNode {
  type: number;
  tag?: string;
  children?: unknown;
  props?: TemplateProp[];
}

interface TemplateProp {
  type: number;
  name: string;
  value?: { content: string };
  arg?: { content?: string };
}

type ElementNode = TemplateNode & { tag: string; children: TemplateNode[]; props: TemplateProp[] };

const ELEMENT = 1;
const ATTRIBUTE = 6;

const source = await Bun.file(new URL("../../app/pages/edit.vue", import.meta.url)).text();
const template = parse(source).descriptor.template!.ast! as unknown as { children: TemplateNode[] };
const config = (await import("../../app/app.config")).default as { ui: Record<string, { slots?: Record<string, string>; variants?: Record<string, unknown> }> };

function isElement(node: TemplateNode): node is ElementNode {
  return node.type === ELEMENT;
}

function elements(nodes: TemplateNode[]): ElementNode[] {
  return nodes.flatMap((node) => {
    const children = Array.isArray(node.children) ? elements(node.children as TemplateNode[]) : [];

    return isElement(node) ? [node, ...children] : children;
  });
}

function attribute(element: ElementNode, name: string): string | undefined {
  return element.props.find((prop) => prop.type === ATTRIBUTE && prop.name === name)?.value?.content;
}

function slotOf(element: ElementNode, name: string): ElementNode | undefined {
  return element.children.filter(isElement).find((child) => child.tag === "template" && child.props.some((prop) => prop.name === "slot" && prop.arg?.content === name));
}

const all = elements(template.children);
const toolbar = all.find((element) => element.tag === "UDashboardToolbar")!;

describe("the editor's bars", () => {
  test("the top bar holds nothing beside the title but logging out at its right end", () => {
    const navbar = all.find((element) => element.tag === "UDashboardNavbar" && slotOf(element, "right") !== undefined)!;
    const parts = navbar.children.filter(isElement);
    const buttons = elements(slotOf(navbar, "right")!.children).filter((element) => element.tag === "UButton");

    expect(parts).toHaveLength(1);
    expect(buttons.map((button) => attribute(button, "icon"))).toEqual(["i-material-symbols-logout"]);
  });

  test("leaving a part's inside stands first at the toolbar's left end", () => {
    const first = elements(slotOf(toolbar, "left")!.children).find((element) => element.tag === "UButton")!;

    expect(attribute(first, "icon")).toBe("i-material-symbols-arrow-back");
  });

  test("every control in the toolbar is the library's small one", () => {
    const controls = elements(toolbar.children).filter((element) => element.tag === "UButton" || element.tag === "USelect");

    expect(controls.length).toBeGreaterThan(0);
    expect(controls.filter((control) => attribute(control, "size") !== "sm")).toEqual([]);
  });

  test("the theme sizes no icon button and stretches nothing in a toolbar", () => {
    expect(config.ui.button?.variants?.square).toBeUndefined();
    expect(config.ui.dashboardToolbar?.slots?.root).not.toContain("--");
  });
});
