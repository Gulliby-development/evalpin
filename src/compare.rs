#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Vs {
    Better,
    Same,
    Worse,
    New,
    Gone,
}

impl Vs {
    pub fn word(self) -> &'static str {
        match self {
            Self::Better => "better",
            Self::Same => "same",
            Self::Worse => "worse",
            Self::New => "new",
            Self::Gone => "gone",
        }
    }
}

pub fn vs(now: Option<(usize, usize)>, before: Option<(usize, usize)>) -> Vs {
    let Some((now_passed, now_total)) = now else {
        return if before.is_none() { Vs::New } else { Vs::Gone };
    };
    let Some((before_passed, before_total)) = before else {
        return Vs::New;
    };

    let u64_products = (
        u64::try_from(now_passed).ok().and_then(|passed| {
            u64::try_from(before_total)
                .ok()
                .and_then(|total| passed.checked_mul(total))
        }),
        u64::try_from(before_passed).ok().and_then(|passed| {
            u64::try_from(now_total)
                .ok()
                .and_then(|total| passed.checked_mul(total))
        }),
    );

    let ordering = match u64_products {
        (Some(now_product), Some(before_product)) => now_product.cmp(&before_product),
        _ => {
            let now_product = (now_passed as u128) * (before_total as u128);
            let before_product = (before_passed as u128) * (now_total as u128);
            now_product.cmp(&before_product)
        }
    };

    match ordering {
        std::cmp::Ordering::Greater => Vs::Better,
        std::cmp::Ordering::Equal => Vs::Same,
        std::cmp::Ordering::Less => Vs::Worse,
    }
}
