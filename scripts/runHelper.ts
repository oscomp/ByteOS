export function parseArgsString(cmdLine: string): string[] {
    return cmdLine.trim().split(/\s+/);
}

export function runCommand(cmdLine: string): Deno.Command {
    const args = parseArgsString(cmdLine);
    return new Deno.Command(args[0], {
        args: args.slice(1)
    });
}

export function testParse() {
    const args = parseArgsString("qemu-system-x86_64 --arch x86_64 --log-level info");
    console.log(args);
}
