// Copyright (c) 2019 Ant Financial
//
// SPDX-License-Identifier: Apache-2.0
//

// #![allow(unused_attributes)]
// #![allow(unused_imports)]
// #![allow(unused_variables)]
// #![allow(unused_mut)]
#![allow(dead_code)]
// #![allow(deprecated)]
// #![allow(unused_must_use)]
#![allow(non_upper_case_globals)]
// #![allow(unused_comparisons)]
#[macro_use]
#[cfg(test)]
extern crate serial_test;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate caps;
extern crate protocols;
#[macro_use]
extern crate scopeguard;
extern crate prctl;
#[macro_use]
extern crate lazy_static;
extern crate libc;
extern crate protobuf;
#[macro_use]
extern crate slog;
#[macro_use]
extern crate scan_fmt;
extern crate oci;
extern crate path_absolutize;
extern crate regex;

pub mod capabilities;
pub mod cgroups;
pub mod container;
pub mod mount;
pub mod pipestream;
pub mod process;
pub mod specconv;
pub mod sync;
pub mod sync_with_async;
pub mod utils;
pub mod validator;
// pub mod factory;
//pub mod configs;
// pub mod devices;
// pub mod init;
// pub mod rootfs;
// pub mod capabilities;
// pub mod console;
// pub mod stats;
// pub mod user;
//pub mod intelrdt;

// construtc ociSpec from grpcSpec, which is needed for hook
// execution. since hooks read config.json

use oci::{
    Box as ociBox, Hooks as ociHooks, Linux as ociLinux, LinuxCapabilities as ociLinuxCapabilities,
    Mount as ociMount, POSIXRlimit as ociPOSIXRlimit, Process as ociProcess, Root as ociRoot,
    Spec as ociSpec, User as ociUser,
};
use protocols::oci::{
    Hooks as grpcHooks, Linux as grpcLinux, Mount as grpcMount, Process as grpcProcess,
    Root as grpcRoot, Spec as grpcSpec,
};
use std::collections::HashMap;

pub fn process_grpc_to_oci(p: &grpcProcess) -> ociProcess {
    let console_size = if p.ConsoleSize.is_some() {
        let c = p.ConsoleSize.as_ref().unwrap();
        Some(ociBox {
            height: c.Height,
            width: c.Width,
        })
    } else {
        None
    };

    let user = if p.User.is_some() {
        let u = p.User.as_ref().unwrap();
        ociUser {
            uid: u.UID,
            gid: u.GID,
            additional_gids: u.AdditionalGids.clone(),
            username: u.Username.clone(),
        }
    } else {
        ociUser {
            uid: 0,
            gid: 0,
            additional_gids: vec![],
            username: String::from(""),
        }
    };

    let capabilities = if p.Capabilities.is_some() {
        let cap = p.Capabilities.as_ref().unwrap();

        Some(ociLinuxCapabilities {
            bounding: cap.Bounding.clone().into_vec(),
            effective: cap.Effective.clone().into_vec(),
            inheritable: cap.Inheritable.clone().into_vec(),
            permitted: cap.Permitted.clone().into_vec(),
            ambient: cap.Ambient.clone().into_vec(),
        })
    } else {
        None
    };

    let rlimits = {
        let mut r = Vec::new();
        for lm in p.Rlimits.iter() {
            r.push(ociPOSIXRlimit {
                r#type: lm.Type.clone(),
                hard: lm.Hard,
                soft: lm.Soft,
            });
        }
        r
    };

    ociProcess {
        terminal: p.Terminal,
        console_size,
        user,
        args: p.Args.clone().into_vec(),
        env: p.Env.clone().into_vec(),
        cwd: p.Cwd.clone(),
        capabilities,
        rlimits,
        no_new_privileges: p.NoNewPrivileges,
        apparmor_profile: p.ApparmorProfile.clone(),
        oom_score_adj: Some(p.OOMScoreAdj as i32),
        selinux_label: p.SelinuxLabel.clone(),
    }
}

fn root_grpc_to_oci(root: &grpcRoot) -> ociRoot {
    ociRoot {
        path: root.Path.clone(),
        readonly: root.Readonly,
    }
}

fn mount_grpc_to_oci(m: &grpcMount) -> ociMount {
    ociMount {
        destination: m.destination.clone(),
        r#type: m.field_type.clone(),
        source: m.source.clone(),
        options: m.options.clone().into_vec(),
    }
}

use oci::Hook as ociHook;
use protocols::oci::Hook as grpcHook;

fn hook_grpc_to_oci(h: &[grpcHook]) -> Vec<ociHook> {
    let mut r = Vec::new();
    for e in h.iter() {
        r.push(ociHook {
            path: e.Path.clone(),
            args: e.Args.clone().into_vec(),
            env: e.Env.clone().into_vec(),
            timeout: Some(e.Timeout as i32),
        });
    }
    r
}

fn hooks_grpc_to_oci(h: &grpcHooks) -> ociHooks {
    let prestart = hook_grpc_to_oci(h.Prestart.as_ref());

    let poststart = hook_grpc_to_oci(h.Poststart.as_ref());

    let poststop = hook_grpc_to_oci(h.Poststop.as_ref());

    ociHooks {
        prestart,
        poststart,
        poststop,
    }
}

use oci::{
    LinuxDevice as ociLinuxDevice, LinuxIDMapping as ociLinuxIDMapping,
    LinuxIntelRdt as ociLinuxIntelRdt, LinuxNamespace as ociLinuxNamespace,
    LinuxResources as ociLinuxResources, LinuxSeccomp as ociLinuxSeccomp,
};
use protocols::oci::{
    LinuxIDMapping as grpcLinuxIDMapping, LinuxResources as grpcLinuxResources,
    LinuxSeccomp as grpcLinuxSeccomp,
};

fn idmap_grpc_to_oci(im: &grpcLinuxIDMapping) -> ociLinuxIDMapping {
    ociLinuxIDMapping {
        container_id: im.ContainerID,
        host_id: im.HostID,
        size: im.Size,
    }
}

fn idmaps_grpc_to_oci(ims: &[grpcLinuxIDMapping]) -> Vec<ociLinuxIDMapping> {
    let mut r = Vec::new();
    for im in ims.iter() {
        r.push(idmap_grpc_to_oci(im));
    }
    r
}

use oci::{
    LinuxBlockIO as ociLinuxBlockIO, LinuxBlockIODevice as ociLinuxBlockIODevice,
    LinuxCPU as ociLinuxCPU, LinuxDeviceCgroup as ociLinuxDeviceCgroup,
    LinuxHugepageLimit as ociLinuxHugepageLimit,
    LinuxInterfacePriority as ociLinuxInterfacePriority, LinuxMemory as ociLinuxMemory,
    LinuxNetwork as ociLinuxNetwork, LinuxPids as ociLinuxPids,
    LinuxThrottleDevice as ociLinuxThrottleDevice, LinuxWeightDevice as ociLinuxWeightDevice,
};
use protocols::oci::{
    LinuxBlockIO as grpcLinuxBlockIO, LinuxThrottleDevice as grpcLinuxThrottleDevice,
    LinuxWeightDevice as grpcLinuxWeightDevice,
};

fn throttle_devices_grpc_to_oci(tds: &[grpcLinuxThrottleDevice]) -> Vec<ociLinuxThrottleDevice> {
    let mut r = Vec::new();
    for td in tds.iter() {
        r.push(ociLinuxThrottleDevice {
            blk: ociLinuxBlockIODevice {
                major: td.Major,
                minor: td.Minor,
            },
            rate: td.Rate,
        });
    }
    r
}

fn weight_devices_grpc_to_oci(wds: &[grpcLinuxWeightDevice]) -> Vec<ociLinuxWeightDevice> {
    let mut r = Vec::new();
    for wd in wds.iter() {
        r.push(ociLinuxWeightDevice {
            blk: ociLinuxBlockIODevice {
                major: wd.Major,
                minor: wd.Minor,
            },
            weight: Some(wd.Weight as u16),
            leaf_weight: Some(wd.LeafWeight as u16),
        });
    }
    r
}

fn blockio_grpc_to_oci(blk: &grpcLinuxBlockIO) -> ociLinuxBlockIO {
    let weight_device = weight_devices_grpc_to_oci(blk.WeightDevice.as_ref());
    let throttle_read_bps_device = throttle_devices_grpc_to_oci(blk.ThrottleReadBpsDevice.as_ref());
    let throttle_write_bps_device =
        throttle_devices_grpc_to_oci(blk.ThrottleWriteBpsDevice.as_ref());
    let throttle_read_iops_device =
        throttle_devices_grpc_to_oci(blk.ThrottleReadIOPSDevice.as_ref());
    let throttle_write_iops_device =
        throttle_devices_grpc_to_oci(blk.ThrottleWriteIOPSDevice.as_ref());

    ociLinuxBlockIO {
        weight: Some(blk.Weight as u16),
        leaf_weight: Some(blk.LeafWeight as u16),
        weight_device,
        throttle_read_bps_device,
        throttle_write_bps_device,
        throttle_read_iops_device,
        throttle_write_iops_device,
    }
}

pub fn resources_grpc_to_oci(res: &grpcLinuxResources) -> ociLinuxResources {
    let devices = {
        let mut d = Vec::new();
        for dev in res.Devices.iter() {
            let major = if dev.Major == -1 {
                None
            } else {
                Some(dev.Major)
            };

            let minor = if dev.Minor == -1 {
                None
            } else {
                Some(dev.Minor)
            };
            d.push(ociLinuxDeviceCgroup {
                allow: dev.Allow,
                r#type: dev.Type.clone(),
                major,
                minor,
                access: dev.Access.clone(),
            });
        }
        d
    };

    let memory = if res.Memory.is_some() {
        let mem = res.Memory.as_ref().unwrap();
        Some(ociLinuxMemory {
            limit: Some(mem.Limit),
            reservation: Some(mem.Reservation),
            swap: Some(mem.Swap),
            kernel: Some(mem.Kernel),
            kernel_tcp: Some(mem.KernelTCP),
            swappiness: Some(mem.Swappiness as i64),
            disable_oom_killer: Some(mem.DisableOOMKiller),
        })
    } else {
        None
    };

    let cpu = if res.CPU.is_some() {
        let c = res.CPU.as_ref().unwrap();
        Some(ociLinuxCPU {
            shares: Some(c.Shares),
            quota: Some(c.Quota),
            period: Some(c.Period),
            realtime_runtime: Some(c.RealtimeRuntime),
            realtime_period: Some(c.RealtimePeriod),
            cpus: c.Cpus.clone(),
            mems: c.Mems.clone(),
        })
    } else {
        None
    };

    let pids = if res.Pids.is_some() {
        let p = res.Pids.as_ref().unwrap();
        Some(ociLinuxPids { limit: p.Limit })
    } else {
        None
    };

    let block_io = if res.BlockIO.is_some() {
        let blk = res.BlockIO.as_ref().unwrap();
        // copy LinuxBlockIO
        Some(blockio_grpc_to_oci(blk))
    } else {
        None
    };

    let hugepage_limits = {
        let mut r = Vec::new();
        for hl in res.HugepageLimits.iter() {
            r.push(ociLinuxHugepageLimit {
                page_size: hl.Pagesize.clone(),
                limit: hl.Limit,
            });
        }
        r
    };

    let network = if res.Network.is_some() {
        let net = res.Network.as_ref().unwrap();
        let priorities = {
            let mut r = Vec::new();
            for pr in net.Priorities.iter() {
                r.push(ociLinuxInterfacePriority {
                    name: pr.Name.clone(),
                    priority: pr.Priority,
                });
            }
            r
        };
        Some(ociLinuxNetwork {
            class_id: Some(net.ClassID),
            priorities,
        })
    } else {
        None
    };

    ociLinuxResources {
        devices,
        memory,
        cpu,
        pids,
        block_io,
        hugepage_limits,
        network,
        rdma: HashMap::new(),
    }
}

use oci::{LinuxSeccompArg as ociLinuxSeccompArg, LinuxSyscall as ociLinuxSyscall};

fn seccomp_grpc_to_oci(sec: &grpcLinuxSeccomp) -> ociLinuxSeccomp {
    let syscalls = {
        let mut r = Vec::new();

        for sys in sec.Syscalls.iter() {
            let mut args = Vec::new();

            for arg in sys.Args.iter() {
                args.push(ociLinuxSeccompArg {
                    index: arg.Index as u32,
                    value: arg.Value,
                    value_two: arg.ValueTwo,
                    op: arg.Op.clone(),
                });
            }

            r.push(ociLinuxSyscall {
                names: sys.Names.clone().into_vec(),
                action: sys.Action.clone(),
                args,
            });
        }
        r
    };

    ociLinuxSeccomp {
        default_action: sec.DefaultAction.clone(),
        architectures: sec.Architectures.clone().into_vec(),
        syscalls,
    }
}

fn linux_grpc_to_oci(l: &grpcLinux) -> ociLinux {
    let uid_mappings = idmaps_grpc_to_oci(l.UIDMappings.as_ref());
    let gid_mappings = idmaps_grpc_to_oci(l.GIDMappings.as_ref());

    let resources = if l.Resources.is_some() {
        Some(resources_grpc_to_oci(l.Resources.as_ref().unwrap()))
    } else {
        None
    };

    let seccomp = if l.Seccomp.is_some() {
        Some(seccomp_grpc_to_oci(l.Seccomp.as_ref().unwrap()))
    } else {
        None
    };

    let namespaces = {
        let mut r = Vec::new();

        for ns in l.Namespaces.iter() {
            r.push(ociLinuxNamespace {
                r#type: ns.Type.clone(),
                path: ns.Path.clone(),
            });
        }
        r
    };

    let devices = {
        let mut r = Vec::new();

        for d in l.Devices.iter() {
            r.push(ociLinuxDevice {
                path: d.Path.clone(),
                r#type: d.Type.clone(),
                major: d.Major,
                minor: d.Minor,
                file_mode: Some(d.FileMode),
                uid: Some(d.UID),
                gid: Some(d.GID),
            });
        }
        r
    };

    let intel_rdt = if l.IntelRdt.is_some() {
        let rdt = l.IntelRdt.as_ref().unwrap();

        Some(ociLinuxIntelRdt {
            l3_cache_schema: rdt.L3CacheSchema.clone(),
        })
    } else {
        None
    };

    ociLinux {
        uid_mappings,
        gid_mappings,
        sysctl: l.Sysctl.clone(),
        resources,
        cgroups_path: l.CgroupsPath.clone(),
        namespaces,
        devices,
        seccomp,
        rootfs_propagation: l.RootfsPropagation.clone(),
        masked_paths: l.MaskedPaths.clone().into_vec(),
        readonly_paths: l.ReadonlyPaths.clone().into_vec(),
        mount_label: l.MountLabel.clone(),
        intel_rdt,
    }
}

fn linux_oci_to_grpc(_l: &ociLinux) -> grpcLinux {
    grpcLinux::default()
}

pub fn grpc_to_oci(grpc: &grpcSpec) -> ociSpec {
    // process
    let process = if grpc.Process.is_some() {
        Some(process_grpc_to_oci(grpc.Process.as_ref().unwrap()))
    } else {
        None
    };

    // root
    let root = if grpc.Root.is_some() {
        Some(root_grpc_to_oci(grpc.Root.as_ref().unwrap()))
    } else {
        None
    };

    // mounts
    let mounts = {
        let mut r = Vec::new();
        for m in grpc.Mounts.iter() {
            r.push(mount_grpc_to_oci(m));
        }
        r
    };

    // hooks
    let hooks = if grpc.Hooks.is_some() {
        Some(hooks_grpc_to_oci(grpc.Hooks.as_ref().unwrap()))
    } else {
        None
    };

    // Linux
    let linux = if grpc.Linux.is_some() {
        Some(linux_grpc_to_oci(grpc.Linux.as_ref().unwrap()))
    } else {
        None
    };

    ociSpec {
        version: grpc.Version.clone(),
        process,
        root,
        hostname: grpc.Hostname.clone(),
        mounts,
        hooks,
        annotations: grpc.Annotations.clone(),
        linux,
        solaris: None,
        windows: None,
        vm: None,
    }
}

#[cfg(test)]
mod tests {
    #[allow(unused_macros)]
    #[macro_export]
    macro_rules! skip_if_not_root {
        () => {
            if !nix::unistd::Uid::effective().is_root() {
                println!("INFO: skipping {} which needs root", module_path!());
                return;
            }
        };
    }
}
