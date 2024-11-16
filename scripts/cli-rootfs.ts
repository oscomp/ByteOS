import { Command, CommandOptions } from "https://deno.land/x/cliffy@v1.0.0-rc.4/command/mod.ts";
import { globalArgType } from "./cli-types.ts";
import { runCommand } from "./runHelper.ts";

/**
 * Build the root filesystem image for the kernel.
 * @param options Global arguments for the command.
 */
async function buildRootFS(options: CommandOptions<globalArgType>) {
    // Create disk image
    const ddStatus = await runCommand(`
        dd if=/dev/zero of=mount.img bs=1M count=64
    `).spawn().status;
    if (!ddStatus.success) return;

    // Format the disk image as ext4
    const mkfsStatus = await runCommand(`
        mkfs.ext4 -F -O ^metadata_csum_seed mount.img
    `).spawn().status;
    if (!mkfsStatus.success) return;

    // Mount the disk image to a temporary directory
    // and copy the necessary files into it
    if(!(await Deno.lstat("mount")).isDirectory) {
        await Deno.mkdir("mount");
    };

    await runCommand(`sudo mount mount.img mount`).spawn().status;
    await runCommand(`sudo rsync -r tools/testcase-${options.arch}/ mount`).spawn().status;
    await runCommand(`sudo umount mount`).spawn().status;
}

export const cliCommand = new Command<globalArgType>()
    .description("Build root fs image for the kernel")
    .action(buildRootFS);
