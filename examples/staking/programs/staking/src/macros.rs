/// Shorthand for `ctx.accounts.<field>.to_account_info()`.
/// Mnemonic: ai = account info
///
/// Yields an owned `AccountInfo`; borrow at the call site (`&ai!(ctx, asset)`)
/// when an API wants a reference, e.g. the mpl-core CpiBuilder methods. This
/// keeps one macro covering both the builder calls and struct-literal fields
/// like `MintToChecked { mint: ai!(ctx, rewards_mint), .. }`.
///
/// `ctx` must be passed explicitly: macro hygiene means a bare `ctx` written
/// inside the macro body would resolve at the definition site, not the call
/// site, so it would never see the handler's `ctx`.
macro_rules! ai {
    ($ctx:ident, $field:ident) => {
        $ctx.accounts.$field.to_account_info()
    };
}

pub(crate) use ai;
