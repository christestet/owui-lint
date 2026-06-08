import * as vscode from "vscode";
import { LanguageClient, LanguageClientOptions, ServerOptions, State } from 'vscode-languageclient/node';

let client: LanguageClient | undefined;

export function activate(context: vscode.ExtensionContext): void {
  client = createClient();
  context.subscriptions.push(
    vscode.commands.registerCommand("owui-lint.restartServer", async () => {
      await restart();
    }),
    // Reveal the server's output channel, where window/logMessage entries
    // (startup, lint summaries, errors) and LSP message traces appear.
    vscode.commands.registerCommand("owui-lint.showOutput", () => {
      client?.outputChannel.show();
    }),
  );
  void startClient(client);
}

export async function deactivate(): Promise<void> {
  await stopClient(client);
  client = undefined;
}

function createClient(): LanguageClient {
  const config = vscode.workspace.getConfiguration('owui-lint');
  const command = resolveExecutablePath(config.get<string>('path', 'owui-lint'));

  // owui-lint complements Pylance/Pyright; it only adds Open WebUI-specific
  // diagnostics, hovers, quick-fixes and scaffolding snippets.
  // Stdio is the default transport for an executable server. We intentionally
  // omit an explicit `transport` so the client does not append a `--stdio`
  // argument, which the owui-lint `server` subcommand does not require.
  const serverOptions: ServerOptions = {
    run: { command, args: ['server'] },
    debug: { command, args: ['server'] },
  };

  const clientOptions: LanguageClientOptions = {
    documentSelector: [{ scheme: 'file', language: 'python' }],
  };

  return new LanguageClient('owui-lint', 'owui-lint', serverOptions, clientOptions);
}

function resolveExecutablePath(configuredPath: string): string {
  const value = configuredPath.trim();
  if (value === '' || value === 'owui-lint') {
    return 'owui-lint';
  }

  const workspaceFolder = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
  const expanded = workspaceFolder ? value.replaceAll('${workspaceFolder}', workspaceFolder) : value;

  if (isAbsolutePath(expanded)) {
    return expanded;
  }

  if (workspaceFolder && looksLikeRelativePath(expanded)) {
    return vscode.Uri.joinPath(vscode.Uri.file(workspaceFolder), expanded.replace(/\\/g, '/')).fsPath;
  }

  return expanded;
}

function isAbsolutePath(value: string): boolean {
  return value.startsWith('/') || /^[A-Za-z]:[\\/]/.test(value) || value.startsWith('\\\\');
}

function looksLikeRelativePath(value: string): boolean {
  return value.startsWith('.') || value.includes('/') || value.includes('\\');
}

async function restart(): Promise<void> {
  await stopClient(client);
  client = createClient();
  void startClient(client);
}

async function startClient(languageClient: LanguageClient): Promise<void> {
  try {
    await languageClient.start();
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    void vscode.window.showErrorMessage(`owui-lint language server failed to start: ${message}`);
  }
}

async function stopClient(languageClient: LanguageClient | undefined): Promise<void> {
  if (!languageClient) {
    return;
  }

  // If the client is starting, wait for it to reach Running state before stopping
  if (languageClient.state === State.Starting) {
    await new Promise<void>((resolve) => {
      let timeoutHandle: ReturnType<typeof setTimeout> | undefined;
      let disposeListener: vscode.Disposable | undefined;

      timeoutHandle = setTimeout(() => {
        disposeListener?.dispose();
        resolve();
      }, 5000); // 5 second timeout

      disposeListener = languageClient.onDidChangeState((event: { oldState: State; newState: State }) => {
        if (event.newState === State.Running) {
          if (timeoutHandle !== undefined) {
            clearTimeout(timeoutHandle);
          }
          disposeListener?.dispose();
          resolve();
        }
      });
    });
  }

  // Only stop if the client is running
  if (languageClient.state !== State.Running) {
    return;
  }

  await languageClient.stop();
}
