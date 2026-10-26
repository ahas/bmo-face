import { describe, expect, test } from "bun:test";

// The console screen's frame (docs/face.md, 「The console screen」): under a console that offers its title bar the
// editor draws no dashboard group and no bar of its own, and its route asks for the console's bar.

const read = (path: string) => Bun.file(new URL(`../../${path}`, import.meta.url)).text();

describe("the console screen's frame", () => {
  test("the route asks for the console's title bar", async () => {
    expect(await read("app/console/bundle.ts")).toContain("meta: { consoleTitleBar: true }");
  });

  test("the editor reads the offer and stands its panels in the console's group, drawing no bar", async () => {
    const page = await read("app/pages/edit.vue");

    expect(page).toContain('const barOffered = inject("bmo-console-title-bar", null) !== null;');
    expect(page).toContain('<EditFrame v-else-if="editor !== undefined" :offered="barOffered">');
    expect(page.match(/<UDashboardNavbar v-if="!barOffered"/g)?.length).toBe(2);
    expect(page).not.toContain("<UDashboardGroup");

    const frame = await read("app/components/EditFrame.vue");

    expect(frame).toContain('<UDashboardGroup v-if="!props.offered"');
    expect(frame).toContain('<div v-else class="flex min-h-0 min-w-0 flex-1">');
  });
});
