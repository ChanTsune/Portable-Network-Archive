use super::re::{
    bsd::{SubstitutionRule, SubstitutionRules},
    gnu::{TransformRule, TransformRules},
};

#[derive(Clone, Debug)]
pub(crate) enum PathTransformers {
    BsdSubstitutions(SubstitutionRules),
    GnuTransforms(TransformRules),
}

impl PathTransformers {
    pub(crate) fn new(
        substitutions: Option<Vec<SubstitutionRule>>,
        transforms: Option<Vec<TransformRule>>,
    ) -> Option<Self> {
        if let Some(s) = substitutions {
            Some(Self::BsdSubstitutions(SubstitutionRules::new(s)))
        } else {
            transforms.map(|t| Self::GnuTransforms(TransformRules::new(t)))
        }
    }
    #[inline]
    pub(crate) fn apply(
        &self,
        input: impl Into<String>,
        is_symlink: bool,
        is_hardlink: bool,
    ) -> String {
        match self {
            Self::BsdSubstitutions(s) => s.apply(input, is_symlink, is_hardlink),
            Self::GnuTransforms(t) => t.apply(input, is_symlink, is_hardlink),
        }
    }
}
