#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookCategory {
    Commit,
    Merge,
    Rebase,
    Push,
    Receive,
    Email,
    Misc,
}

impl HookCategory {
    pub fn label(self) -> &'static str {
        match self {
            Self::Commit => "Commit",
            Self::Merge => "Merge",
            Self::Rebase => "Rebase",
            Self::Push => "Push",
            Self::Receive => "Receive (server-side)",
            Self::Email => "Email / Patch",
            Self::Misc => "Misc",
        }
    }
}

pub struct HookInfo {
    pub name: &'static str,
    pub description: &'static str,
    pub category: HookCategory,
}

pub const ALL_HOOKS: &[HookInfo] = &[
    // Commit
    HookInfo {
        name: "pre-commit",
        description: "Runs before commit message editor; abort with non-zero exit.",
        category: HookCategory::Commit,
    },
    HookInfo {
        name: "prepare-commit-msg",
        description: "Runs after default message is prepared, before editor starts.",
        category: HookCategory::Commit,
    },
    HookInfo {
        name: "commit-msg",
        description: "Validates/normalises the commit message. Receives message file path as $1.",
        category: HookCategory::Commit,
    },
    HookInfo {
        name: "post-commit",
        description: "Notification hook after a commit is made. Cannot affect outcome.",
        category: HookCategory::Commit,
    },
    // Merge
    HookInfo {
        name: "pre-merge-commit",
        description: "Runs after merge completes, before merge commit is created.",
        category: HookCategory::Merge,
    },
    HookInfo {
        name: "post-merge",
        description: "Notification hook after git-merge / git-pull. Cannot affect outcome.",
        category: HookCategory::Merge,
    },
    // Rebase
    HookInfo {
        name: "pre-rebase",
        description: "Runs before git-rebase; abort with non-zero exit.",
        category: HookCategory::Rebase,
    },
    HookInfo {
        name: "post-rewrite",
        description: "Runs after commits are rewritten (amend, rebase).",
        category: HookCategory::Rebase,
    },
    // Push (client-side)
    HookInfo {
        name: "pre-push",
        description: "Runs before git-push; abort with non-zero exit.",
        category: HookCategory::Push,
    },
    HookInfo {
        name: "push-to-checkout",
        description: "Overrides default behaviour when pushing to a checked-out branch.",
        category: HookCategory::Push,
    },
    // Receive (server-side)
    HookInfo {
        name: "pre-receive",
        description: "Server-side: runs once before updating refs. Abort with non-zero exit.",
        category: HookCategory::Receive,
    },
    HookInfo {
        name: "update",
        description: "Server-side: runs once per ref being updated.",
        category: HookCategory::Receive,
    },
    HookInfo {
        name: "proc-receive",
        description: "Server-side: handles commands matched by receive.procReceiveRefs.",
        category: HookCategory::Receive,
    },
    HookInfo {
        name: "post-receive",
        description: "Server-side: notification after all refs are updated.",
        category: HookCategory::Receive,
    },
    HookInfo {
        name: "post-update",
        description: "Server-side: notification after all refs are updated (receives ref names).",
        category: HookCategory::Receive,
    },
    // Email / Patch (git-am)
    HookInfo {
        name: "applypatch-msg",
        description: "Validates/normalises commit message from applied patch.",
        category: HookCategory::Email,
    },
    HookInfo {
        name: "pre-applypatch",
        description: "Runs after patch is applied but before committing.",
        category: HookCategory::Email,
    },
    HookInfo {
        name: "post-applypatch",
        description: "Notification after patch is applied and committed.",
        category: HookCategory::Email,
    },
    HookInfo {
        name: "sendemail-validate",
        description: "Validates patch emails before git-send-email sends them.",
        category: HookCategory::Email,
    },
    // Misc
    HookInfo {
        name: "post-checkout",
        description: "Runs after git-checkout or git-switch; receives old HEAD, new HEAD, branch flag.",
        category: HookCategory::Misc,
    },
    HookInfo {
        name: "reference-transaction",
        description: "Runs on every reference transaction (prepared/committed/aborted).",
        category: HookCategory::Misc,
    },
    HookInfo {
        name: "pre-auto-gc",
        description: "Runs before git gc --auto; abort with non-zero exit.",
        category: HookCategory::Misc,
    },
    HookInfo {
        name: "fsmonitor-watchman",
        description: "Used with core.fsmonitor for filesystem change monitoring.",
        category: HookCategory::Misc,
    },
    HookInfo {
        name: "post-index-change",
        description: "Runs after the index is written.",
        category: HookCategory::Misc,
    },
    // p4
    HookInfo {
        name: "p4-changelist",
        description: "git-p4 submit: validates/normalises changelist message.",
        category: HookCategory::Misc,
    },
    HookInfo {
        name: "p4-prepare-changelist",
        description: "git-p4 submit: runs before changelist editor starts.",
        category: HookCategory::Misc,
    },
    HookInfo {
        name: "p4-post-changelist",
        description: "git-p4 submit: notification after successful submit.",
        category: HookCategory::Misc,
    },
    HookInfo {
        name: "p4-pre-submit",
        description: "git-p4 submit: runs before submission; abort with non-zero exit.",
        category: HookCategory::Misc,
    },
];

pub fn find_hook(name: &str) -> Option<&'static HookInfo> {
    ALL_HOOKS.iter().find(|h| h.name == name)
}
