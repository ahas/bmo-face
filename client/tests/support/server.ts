import { Database } from "bun:sqlite";
import { mkdirSync } from "node:fs";
import { join, resolve } from "node:path";

import type { Subprocess } from "bun";

// The face's own server as a test runs it: the binary Cargo builds, started over a scratch folder that
// holds its data and an archive of its own, answering on the port it chooses. The test that starts it
// stops it.

/** The screen's project, where its builds run. */
export const CLIENT = resolve(import.meta.dir, "../..");

/** The repository's root, where the server's crate builds. */
export const REPOSITORY = resolve(CLIENT, "..");

/** A web account the archive holds, which the editor signs in with. */
export interface Account {
  name: string;
  password: string;
}

/** What a server is started over: the scratch folder, the screen it serves and the console's bundle and credential. */
export interface ServerSetting {
  scratch: string;
  screen: string;
  console?: string;
  token?: string;
  accounts?: Account[];
}

/** A started server: the address its routes stand under, and the way to take it down. */
export interface FaceServer {
  base: string;
  stop: () => Promise<void>;
}

/** Runs a command to its end and answers what it printed; a failure throws with what it said. */
export function run(command: string[], cwd: string, env: Record<string, string> = {}): string {
  const done = Bun.spawnSync(command, { cwd, env: { ...process.env, ...env }, stdout: "pipe", stderr: "pipe" });

  if (done.exitCode !== 0) {
    throw new Error(`${command.join(" ")} failed: ${done.stderr.toString()}${done.stdout.toString()}`);
  }

  return done.stdout.toString();
}

// The server's binary as Cargo builds it, wherever the build directory stands.
function serverBinary(): string {
  const said = run(["cargo", "build", "--quiet", "--locked", "--bin", "bmo-face", "--message-format=json"], REPOSITORY);
  const built = said
    .split("\n")
    .filter((line) => line.startsWith("{"))
    .map((line) => JSON.parse(line) as { reason?: string; executable?: string | null; target?: { name?: string } })
    .find((message) => message.reason === "compiler-artifact" && message.target?.name === "bmo-face" && message.executable);

  if (!built?.executable) {
    throw new Error("cargo built no bmo-face binary");
  }

  return built.executable;
}

// An archive holding the web accounts the server reads, each password kept as an argon2id hash.
async function writeArchive(path: string, accounts: Account[]): Promise<void> {
  const archive = new Database(path, { create: true });

  archive.run("CREATE TABLE web_users (name TEXT PRIMARY KEY, password_hash TEXT NOT NULL)");

  for (const account of accounts) {
    archive.run("INSERT INTO web_users (name, password_hash) VALUES (?, ?)", [account.name, await Bun.password.hash(account.password, "argon2id")]);
  }

  archive.close();
}

// The address the server names as it starts answering, read from what it prints.
async function listeningBase(server: Subprocess<"ignore", "pipe", "inherit">): Promise<string> {
  const decoder = new TextDecoder();
  let said = "";

  for await (const chunk of server.stdout) {
    said += decoder.decode(chunk);
    const port = /BMO face: http:\/\/127\.0\.0\.1:(\d+)\//.exec(said)?.[1];

    if (port !== undefined) {
      return `http://127.0.0.1:${port}/bmo-face`;
    }
  }

  throw new Error(`the server never listened; it said ${said}`);
}

/** Builds the server and starts it over the scratch folder, answering once it listens. */
export async function startServer(setting: ServerSetting): Promise<FaceServer> {
  const binary = serverBinary();
  const archive = join(setting.scratch, "archive.db");

  mkdirSync(join(setting.scratch, "data"));
  await writeArchive(archive, setting.accounts ?? []);

  const server = Bun.spawn([binary], {
    env: {
      ...process.env,
      BMO_FACE_DATA: join(setting.scratch, "data"),
      BMO_FACE_PORT: "0",
      BMO_FACE_STATIC: setting.screen,
      BMO_FACE_CONSOLE: setting.console ?? join(setting.scratch, "console"),
      BMO_FACE_TOKEN: setting.token ?? "",
      BMO_DB_PATH: archive,
      BMO_ARCHIVE_URL: "",
      BMO_CONSOLE_URL: "http://[::1]:9/bmo",
    },
    stdin: "ignore",
    stdout: "pipe",
    stderr: "inherit",
  });

  const stop = async (): Promise<void> => {
    server.kill();
    await server.exited;
  };

  try {
    return { base: await listeningBase(server), stop };
  } catch (fault) {
    await stop();
    throw fault;
  }
}
