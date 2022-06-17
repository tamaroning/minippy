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

use hir::{BinOpKind, Expr, ExprKind};
use rustc_ast::ast::LitKind;
use rustc_hir as hir;
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
        ];

        rustc_driver::RunCompiler::new(&args, &mut MinippyCallBacks).run()
    }));
}

struct MinippyCallBacks;

impl rustc_driver::Callbacks for MinippyCallBacks {
    fn config(&mut self, config: &mut rustc_interface::Config) {
        config.register_lints = Some(Box::new(move |_sess, lint_store| {
            lint_store.register_late_pass(|| Box::new(AddZero));
            lint_store.register_late_pass(|| Box::new(UnwrapUsed));
        }));
    }

    fn after_analysis<'tcx>(
        &mut self,
        _compiler: &rustc_interface::interface::Compiler,
        _queries: &'tcx rustc_interface::Queries<'tcx>,
    ) -> rustc_driver::Compilation {
        rustc_driver::Compilation::Stop
    }
}

// ここからadd zero lintの定義

// おまじない
declare_tool_lint! {
    pub crate::ADD_ZERO,
    Warn, // lintのレベル
    "", // lintの説明(今回は省略)
    report_in_external_macro: true
}

struct AddZero;
// おまじない
impl_lint_pass!(AddZero => [ADD_ZERO]);

// 式がリテラルの0かチェックする
fn is_lit_zero(expr: &Expr) -> bool {
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
        // マクロ展開されたコードはリントしない
        if expr.span.from_expansion() {
            return;
        }
        // 二項演算かつ、左辺もしくは右辺がリテラルの0であるならば、
        if let ExprKind::Binary(binop, lhs, rhs) = expr.kind
            && BinOpKind::Add == binop.node
            && (is_lit_zero(lhs) || is_lit_zero(rhs))
        {
            // 警告を表示する
            cx.struct_span_lint(ADD_ZERO, expr.span, |diag| {
                let mut diag = diag.build("Ineffective operation");
                diag.emit();
            });
        }
    }
}

// ここからunwrap used lintの定義

// おまじない
declare_tool_lint! {
    pub crate::UNWRAP_USED,
    Warn,
    "",
    report_in_external_macro: true
}

struct UnwrapUsed;
impl_lint_pass!(UnwrapUsed => [UNWRAP_USED]);

impl<'tcx> LateLintPass<'tcx> for UnwrapUsed {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if let ExprKind::MethodCall(method_path, /* args */ _, /* span */_) = expr.kind
            && method_path.ident.name == rustc_span::symbol::sym::unwrap
        {
            cx.struct_span_lint(ADD_ZERO, expr.span, |diag| {
                let mut diag = diag.build("`unwrap` is used here");
                diag.emit();
            });
        }
    }
}
