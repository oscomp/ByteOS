use super::{stack::init_task_stack, UserTask};
use crate::{
    consts::USER_DYN_ADDR,
    tasks::{elf::ElfExtra, MemType},
};
use alloc::{
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use core::ops::{Add, Mul};
use devices::{frame_alloc_much, PAGE_SIZE};
use fs::{file::File, pathbuf::PathBuf};
use libc_types::fcntl::OpenFlags;
use syscalls::Errno;
use xmas_elf::program::{SegmentData, Type};

pub fn exec_with_process(
    task: Arc<UserTask>,
    curr_dir: PathBuf,
    path: String,
    args: Vec<String>,
    envp: Vec<String>,
) -> Result<Arc<UserTask>, Errno> {
    // copy args, avoid free before pushing.
    // let path = String::from(path);
    let path = curr_dir.join(&path);

    let user_task = task.clone();
    user_task.pcb.lock().memset.clear();
    user_task.page_table.restore();
    user_task.page_table.change();

    // TODO: 运行程序的时候，判断当前的路径
    let file = File::open_link(path.clone(), OpenFlags::RDONLY)
        .map(Arc::new)?
        .clone();
    let file_size = file.file_size()?;
    let frame_ppn = frame_alloc_much(file_size.div_ceil(PAGE_SIZE));
    let buffer = frame_ppn.as_ref().unwrap()[0].slice_mut_with_len(file_size);
    let rsize = file.readat(0, buffer)?;
    assert_eq!(rsize, file_size);
    // flush_dcache_range();
    // 读取elf信息
    let elf = if let Ok(elf) = xmas_elf::ElfFile::new(&buffer) {
        elf
    } else {
        let mut new_args = vec!["busybox".to_string(), "sh".to_string()];
        args.iter().for_each(|x| new_args.push(x.clone()));
        return exec_with_process(task, curr_dir, String::from("busybox"), new_args, envp);
    };
    let elf_header = elf.header;

    let entry_point = elf.header.pt2.entry_point() as usize;
    // this assert ensures that the file is elf file.
    assert_eq!(
        elf_header.pt1.magic,
        [0x7f, 0x45, 0x4c, 0x46],
        "invalid elf!"
    );
    // WARRNING: this convert async task to user task.
    let user_task = task.clone();

    // check if it is libc, dlopen, it needs recurit.
    let header = elf
        .program_iter()
        .find(|ph| ph.get_type() == Ok(Type::Interp));
    if let Some(header) = header {
        if let Ok(SegmentData::Undefined(_data)) = header.get_data(&elf) {
            drop(frame_ppn);
            let mut new_args = vec![String::from("libc.so")];
            new_args.extend(args);
            return exec_with_process(task, curr_dir, new_args[0].clone(), new_args, envp);
        }
    }

    // 获取程序所有段之后的内存，4K 对齐后作为堆底
    let heap_bottom = elf
        .program_iter()
        .map(|x| (x.virtual_addr() + x.mem_size()) as usize)
        .max()
        .unwrap()
        .div_ceil(PAGE_SIZE)
        .mul(PAGE_SIZE);

    let base = elf.relocate(USER_DYN_ADDR).unwrap_or(0);
    init_task_stack(
        user_task.clone(),
        args,
        base,
        &path.path(),
        entry_point,
        elf_header.pt2.ph_count() as usize,
        elf_header.pt2.ph_entry_size() as usize,
        elf.get_ph_addr().unwrap_or(0) as usize,
        heap_bottom,
    );

    // map sections.
    elf.program_iter()
        .filter(|x| x.get_type().unwrap() == xmas_elf::program::Type::Load)
        .for_each(|ph| {
            let file_size = ph.file_size() as usize;
            let mem_size = ph.mem_size() as usize;
            let offset = ph.offset() as usize;
            let virt_addr = base + ph.virtual_addr() as usize;
            let vpn = virt_addr / PAGE_SIZE;

            let page_count = (virt_addr + mem_size).div_ceil(PAGE_SIZE) - vpn;
            let ppn_start =
                user_task.frame_alloc(va!(virt_addr).floor(), MemType::CodeSection, page_count);
            let page_space = va!(virt_addr).slice_mut_with_len(file_size);
            let ppn_space = ppn_start
                .expect("not have enough memory")
                .add(virt_addr % PAGE_SIZE)
                .slice_mut_with_len(file_size);

            page_space.copy_from_slice(&buffer[offset..offset + file_size]);
            assert_eq!(ppn_space, page_space);
            assert_eq!(&buffer[offset..offset + file_size], ppn_space);
            assert_eq!(&buffer[offset..offset + file_size], page_space);
        });
    Ok(user_task)
}
