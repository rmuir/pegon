import { type Executable, LanguageClient } from 'vscode-languageclient/node';
import { type ExtensionContext, Uri } from 'vscode';

let client: LanguageClient | undefined;

export async function activate(context: ExtensionContext): Promise<void>  {
  const binary = process.platform === 'win32' ? 'pegon.exe' : 'pegon';
  const location = Uri.joinPath(context.extensionUri, 'bin', binary);
  const executable: Executable = {
    args: [ 'server' ],
    command: process.env['PEGON_SERVER_PATH'] ?? location.fsPath,
  };

  client = new LanguageClient(
    'pegon',
    'pegon',
    {
      debug: executable,
      run: executable
    },
    {
      documentSelector: [{ language: 'java', scheme: 'file' }],
    }
  );
  await client.start();
}

export async function deactivate() : Promise<void> {
  await client?.stop();
}
