import { describe, expect, test } from "bun:test";

import { parse } from "vue/compiler-sfc";

// Where the editor's controls stand and at what size (docs/face.md, 「The screens' shared style」): the
// top bar holds the screen's name and logging out alone, the actions stand in the toolbar over the
// canvas with leaving a part's inside at its left end and the one create button at its right end,
// nothing else on the editor creates, and every control in that toolbar is the library's small one
// at its own size, the theme setting no size of its own on an icon button. Every list a value is chosen
// from is the one select, never the library's bare.

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

// A screen file's template, as the compiler reads it.
async function templateOf(path: string): Promise<{ children: TemplateNode[] }> {
  const source = await Bun.file(new URL(path, import.meta.url)).text();

  return parse(source).descriptor.template!.ast! as unknown as { children: TemplateNode[] };
}

const client = new URL("../..", import.meta.url).pathname;
const template = await templateOf("../../app/pages/edit.vue");
const fields = await templateOf("../../app/components/PartFields.vue");
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

// Whether a prop is set, plainly or bound.
function names(element: ElementNode, name: string): boolean {
  return element.props.some((prop) => (prop.type === ATTRIBUTE ? prop.name === name : prop.arg?.content === name));
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

  test("the one create button stands at the toolbar's right end", () => {
    const right = slotOf(toolbar, "right")!.children.filter(isElement);
    const creating = all.filter((element) => element.tag === "CreateButton");

    expect(creating).toHaveLength(1);
    expect(right.at(-1)!.tag).toBe("UDropdownMenu");
    expect(right.at(-1)!.children.filter(isElement).map((child) => child.tag)).toEqual(["CreateButton"]);
  });

  test("nothing else on the editor creates, and no button carries the add mark with a word", () => {
    const buttons = [...all, ...elements(fields.children)].filter((element) => element.tag === "UButton");
    const adding = buttons.filter((button) => /(^|-)add(-|$)/.test(attribute(button, "icon") ?? ""));

    expect(adding.filter((button) => all.includes(button))).toEqual([]);
    expect(adding.filter((button) => names(button, "label"))).toEqual([]);
  });

  test("every control in the toolbar is the library's small one", () => {
    const controls = elements(toolbar.children).filter((element) => ["UButton", "ChoiceSelect", "CreateButton"].includes(element.tag));

    expect(controls.length).toBeGreaterThan(0);
    expect(controls.filter((control) => attribute(control, "size") !== "sm")).toEqual([]);
  });

  test("every value is chosen in the one select, by the value its choices carry", async () => {
    const screens = await Promise.all([...new Bun.Glob("app/**/*.vue").scanSync({ cwd: client })].filter((path) => path !== "app/components/ChoiceSelect.vue").map((path) => templateOf(`../../${path}`)));
    const bare = screens.flatMap((screen) => elements(screen.children)).filter((element) => ["USelect", "USelectMenu", "UInputMenu"].includes(element.tag));
    const selects = [...all, ...elements(fields.children)].filter((element) => element.tag === "ChoiceSelect");

    expect(screens.length).toBeGreaterThan(2);
    expect(bare).toEqual([]);
    expect(selects).toHaveLength(7);
    expect(selects.filter((select) => attribute(select, "value-key") !== "value")).toEqual([]);
  });

  test("the theme sizes no icon button and stretches nothing in a toolbar", () => {
    expect(config.ui.button?.variants?.square).toBeUndefined();
    expect(config.ui.dashboardToolbar?.slots?.root).not.toContain("--");
  });
});
