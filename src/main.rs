#![feature(rustc_private, let_chains)]

extern crate rustc_ast;
extern crate rustc_driver;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_interface;
extern crate rustc_lint;
extern crate rustc_lint_defs;
extern crate rustc_session;
extern crate rustc_span;

use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_lint_defs::impl_lint_pass;
use rustc_session::declare_tool_lint;
use std::{process, str};

const USAGE: &str = r#"Usage: minippy INPUT"#;

fn main() {
    println!("{USAGE}");

    rustc_driver::init_rustc_env_logger();
    std::process::exit(rustc_driver::catch_with_exit_code(move || {
        let out = process::Command::new("rustc")
            .arg("--print=sysroot")
            .current_dir(".")
            .output()
            .unwrap();
        let sys_root = str::from_utf8(&out.stdout).unwrap().trim().to_string();

        let orig_args: Vec<String> = std::env::args().collect();
        let filepath = orig_args.last().unwrap().to_string();

        let args: Vec<String> = vec![
            "rustc".to_string(),
            filepath,
            "--sysroot".to_string(),
            sys_root,
            "--out-dir".to_string(),
            "./.minippy".to_string(),
        ];

        rustc_driver::RunCompiler::new(&args, &mut MinippyCallBacks).run()
    }));
}

struct MinippyCallBacks;

impl rustc_driver::Callbacks for MinippyCallBacks {
    fn config(&mut self, config: &mut rustc_interface::Config) {
        config.register_lints = Some(Box::new(move |_sess, lint_store| {
            lint_store.register_late_pass(|| Box::new(AddZero));
        }));
    }
}

declare_tool_lint! {
    pub crate::ADD_ZERO,
    Warn,
    "",
    report_in_external_macro: true
}

struct AddZero;
impl_lint_pass!(AddZero => [ADD_ZERO]);

use hir::{BinOpKind, Expr, ExprKind};
use rustc_ast::ast::LitKind;
use rustc_hir as hir;

fn is_const_zero(expr: &Expr) -> bool {
    if let ExprKind::Lit(lit) = &expr.kind
        && let LitKind::Int(0, ..) = lit.node
    {
        true
    } else {
        false
    }
}

impl<'tcx> LateLintPass<'tcx> for AddZero {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if expr.span.from_expansion() {
            return;
        }

        if let ExprKind::Binary(binop, lhs, rhs) = expr.kind
            && BinOpKind::Add == binop.node
            && (is_const_zero(lhs) || is_const_zero(rhs))
        {
            cx.struct_span_lint(ADD_ZERO, expr.span, |diag| {
                let mut diag = diag.build("Uneffective operation");
                diag.emit();
            });
        }
    }
}
