// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! View filters, their per-view overrides and view presets: the display rules a model view
//! applies on top of its plane and depth.
//!
//! A [`ViewFilter`] is a named list of conditions over the V14 fields (`type`, `name`, `class`
//! or a user attribute), all of which or any of which must hold. It is authored as rows, the way
//! a person reads it ("type equals beam", "class equals 2"), and turned into the V14 rule JSON
//! by `beamcraft_host::views`; the V14 evaluator stays the only matching semantics, so a filter
//! that the evaluator refuses (an unknown attribute, a literal of the wrong type) is refused when
//! it is evaluated, never silently matched.
//!
//! A [`FilterOverride`] binds a filter to one view: whether what it matches is shown, an
//! optional colour and a transparency. A view lists its overrides in order and the first
//! override whose filter matches an element decides that element's display. A [`ViewPreset`]
//! is a named set of display properties that a view can be filled from, like a part preset
//! fills a new part's form; applying a preset copies its values and keeps no link.
//!
//! Display state only: nothing here reads or writes a store.

use crate::RenderMode;
use crate::filter::Operator;
use crate::views::{MAX_DEPTH_MM, MAX_SPAN_MM, MIN_SPAN_MM, Projection};
use crate::{ElementKind, KindFilter};
use beamcraft_model::command::CommandError;
use beamcraft_model::error::ErrorCode;

/// Filters per view book.
pub const MAX_FILTERS: usize = 64;
/// Conditions per filter.
pub const MAX_CONDITIONS: usize = 16;
/// Filter overrides per view.
pub const MAX_OVERRIDES: usize = 16;
/// View presets per view book.
pub const MAX_PRESETS: usize = 64;
/// Most characters in a filter or preset name.
pub const MAX_LABEL_CHARS: usize = 64;
/// Most characters in a condition's value or attribute name.
pub const MAX_VALUE_CHARS: usize = 128;

fn invalid(field: &'static str, message: &'static str) -> CommandError {
    CommandError::new(ErrorCode::InvalidArgument, field, message)
}

/// Admits a filter or preset name: 1 to 64 characters, no outer spaces, no control characters
/// and no `;`, `|` or `:`, which the command-line lists use as separators.
pub fn validate_label(name: &str, field: &'static str) -> Result<(), CommandError> {
    if name.trim() != name
        || name.is_empty()
        || name.chars().count() > MAX_LABEL_CHARS
        || name
            .chars()
            .any(|c| c.is_control() || matches!(c, ';' | '|' | ':'))
    {
        return Err(invalid(
            field,
            "a name needs 1 to 64 characters without outer spaces, controls, `;`, `|` or `:`",
        ));
    }
    Ok(())
}

/// The field a condition compares.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilterField {
    /// The element type token (`beam`, `column`, `plate`, ...).
    Type,
    /// The element's readable name.
    Name,
    /// The part class (an integer; plates, beams and columns only).
    Class,
    /// A user attribute, by its definition name.
    Attribute(String),
}

impl FilterField {
    /// The stable token: `type`, `name`, `class`, or `attribute:<name>`.
    pub fn token(&self) -> String {
        match self {
            Self::Type => "type".into(),
            Self::Name => "name".into(),
            Self::Class => "class".into(),
            Self::Attribute(name) => format!("attribute:{name}"),
        }
    }

    /// Parses a token written by [`FilterField::token`].
    pub fn parse(token: &str) -> Result<Self, CommandError> {
        match token {
            "type" => Ok(Self::Type),
            "name" => Ok(Self::Name),
            "class" => Ok(Self::Class),
            _ => match token.strip_prefix("attribute:") {
                Some(name) => {
                    let field = Self::Attribute(name.to_owned());
                    field.validate()?;
                    Ok(field)
                }
                None => Err(invalid(
                    "views.filter.field",
                    "a filter field is type, name, class or attribute:<name>",
                )),
            },
        }
    }

    fn validate(&self) -> Result<(), CommandError> {
        if let Self::Attribute(name) = self
            && (name.trim() != name
                || name.is_empty()
                || name.chars().count() > MAX_VALUE_CHARS
                || name.chars().any(char::is_control))
        {
            return Err(invalid(
                "views.filter.field",
                "an attribute name needs 1 to 128 characters without outer spaces or controls",
            ));
        }
        Ok(())
    }
}

/// One row of a filter: `field op value`.
#[derive(Debug, Clone, PartialEq)]
pub struct FilterCondition {
    /// What is compared.
    pub field: FilterField,
    /// How it is compared (the V14 operators).
    pub op: Operator,
    /// The literal as the person typed it: a type token, a name, a class number, or an
    /// attribute value (a number, `true`/`false`, a number with its unit such as `250 mm`, or
    /// text).
    pub value: String,
}

impl FilterCondition {
    /// A condition from its parts.
    pub fn new(field: FilterField, op: Operator, value: &str) -> Self {
        Self {
            field,
            op,
            value: value.trim().to_owned(),
        }
    }

    /// Parses a row written as `field op value`, for example `type eq beam`, `class ge 2` or
    /// `attribute:Fire rating eq R30`. The field runs up to the first word that is an operator
    /// token, so an attribute name may hold spaces; the value is the rest of the text and may
    /// hold spaces too.
    pub fn parse_row(text: &str) -> Result<Self, CommandError> {
        let text = text.trim();
        let words: Vec<&str> = text.split(' ').collect();
        // The operator is the first word that is an operator token after the field.
        let at = words
            .iter()
            .enumerate()
            .skip(1)
            .find(|(_, w)| Operator::parse(w).is_some())
            .map(|(i, _)| i)
            .ok_or_else(|| {
                invalid(
                    "views.filter.condition",
                    "a condition reads `field op value`, for example `type eq beam`",
                )
            })?;
        let field = words.get(..at).map(|w| w.join(" ")).unwrap_or_default();
        let op = words
            .get(at)
            .and_then(|w| Operator::parse(w))
            .ok_or_else(|| invalid("views.filter.op", "unknown operator"))?;
        let value = words
            .get(at.saturating_add(1)..)
            .map(|w| w.join(" "))
            .unwrap_or_default();
        let row = Self::new(FilterField::parse(&field)?, op, &value);
        row.validate()?;
        Ok(row)
    }

    /// The row as [`FilterCondition::parse_row`] reads it.
    pub fn to_row(&self) -> String {
        format!("{} {} {}", self.field.token(), self.op.token(), self.value)
    }

    /// Admits the row: a bounded value, a whole class number, a known type token, and an
    /// operator the field can take (text fields take equality, substring and prefix; the class
    /// takes equality and ordering).
    pub fn validate(&self) -> Result<(), CommandError> {
        self.field.validate()?;
        if self.value.is_empty()
            || self.value.chars().count() > MAX_VALUE_CHARS
            || self.value.chars().any(char::is_control)
        {
            return Err(invalid(
                "views.filter.value",
                "a condition needs a value of 1 to 128 characters",
            ));
        }
        let text_op = matches!(
            self.op,
            Operator::Eq | Operator::Ne | Operator::Contains | Operator::StartsWith
        );
        let ordered_op = !matches!(self.op, Operator::Contains | Operator::StartsWith);
        match self.field {
            FilterField::Type => {
                if !text_op {
                    return Err(invalid(
                        "views.filter.op",
                        "the type takes equals, not equals, contains or starts with",
                    ));
                }
                if matches!(self.op, Operator::Eq | Operator::Ne)
                    && ElementKind::parse(&self.value).is_none()
                {
                    return Err(invalid(
                        "views.filter.value",
                        "the type is an element type such as beam, column or plate",
                    ));
                }
            }
            FilterField::Name => {
                if !text_op {
                    return Err(invalid(
                        "views.filter.op",
                        "the name takes equals, not equals, contains or starts with",
                    ));
                }
            }
            FilterField::Class => {
                if !ordered_op {
                    return Err(invalid(
                        "views.filter.op",
                        "the class takes a comparison, not contains or starts with",
                    ));
                }
                if self.value.parse::<i32>().is_err() {
                    return Err(invalid("views.filter.value", "the class is a whole number"));
                }
            }
            FilterField::Attribute(_) => {}
        }
        Ok(())
    }
}

/// Whether every condition of a filter must hold, or any one of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FilterMatch {
    /// Every condition holds; a filter without conditions matches everything.
    #[default]
    All,
    /// At least one condition holds; a filter without conditions matches nothing.
    Any,
}

impl FilterMatch {
    /// The stable token: `all` or `any`.
    pub const fn token(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Any => "any",
        }
    }

    /// Parses an exact token.
    pub fn parse(token: &str) -> Result<Self, CommandError> {
        match token {
            "all" => Ok(Self::All),
            "any" => Ok(Self::Any),
            _ => Err(invalid("views.filter.match", "a filter matches all or any")),
        }
    }
}

/// A named selection of elements, the rows of conditions a view filter evaluates.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewFilter {
    /// Unique name in the book (case-insensitively).
    pub name: String,
    /// How the conditions combine.
    pub matching: FilterMatch,
    /// The rows, at most [`MAX_CONDITIONS`].
    pub conditions: Vec<FilterCondition>,
}

impl ViewFilter {
    /// A filter of `conditions` that must all hold.
    pub fn new(name: &str, conditions: Vec<FilterCondition>) -> Self {
        Self {
            name: name.trim().to_owned(),
            matching: FilterMatch::All,
            conditions,
        }
    }

    /// Admits the filter: its name and every row.
    pub fn validate(&self) -> Result<(), CommandError> {
        validate_label(&self.name, "views.filter.name")?;
        if self.conditions.len() > MAX_CONDITIONS {
            return Err(CommandError::new(
                ErrorCode::LimitExceeded,
                "views.filter.conditions",
                "a filter has at most 16 conditions",
            ));
        }
        self.conditions
            .iter()
            .try_for_each(FilterCondition::validate)
    }
}

/// How one view displays what a filter matches.
#[derive(Debug, Clone, PartialEq)]
pub struct FilterOverride {
    /// The filter's name in the book.
    pub filter: String,
    /// Whether the matched elements are drawn at all.
    pub visible: bool,
    /// Optional colour of the matched elements, linear RGB in `0..=1`.
    pub color: Option<[f64; 3]>,
    /// 0.0 opaque to 1.0 invisible, applied where the pipeline blends.
    pub transparency: f64,
}

impl FilterOverride {
    /// Shows what `filter` matches as it is: visible, model colours, opaque.
    pub fn new(filter: &str) -> Self {
        Self {
            filter: filter.to_owned(),
            visible: true,
            color: None,
            transparency: 0.0,
        }
    }

    /// Parses `filter|show|colour|transparency`: `show` or `hide`, the colour as `r,g,b` in
    /// `0..=1` or `-` for the model colours, the transparency in `0..=1`. Only the filter name
    /// is required: `Beams` alone shows them as they are, `Beams|hide` hides them.
    pub fn parse_spec(text: &str) -> Result<Self, CommandError> {
        let bad = || {
            invalid(
                "views.override",
                "an override reads `filter|show or hide|r,g,b or -|transparency`",
            )
        };
        let mut parts = text.trim().split('|');
        let mut out = Self::new(parts.next().unwrap_or_default().trim());
        if let Some(visible) = parts.next() {
            out.visible = match visible.trim() {
                "show" => true,
                "hide" => false,
                _ => return Err(bad()),
            };
        }
        if let Some(color) = parts.next().map(str::trim).filter(|c| *c != "-") {
            let values = color
                .split(',')
                .map(|v| v.trim().parse::<f64>().map_err(|_| bad()))
                .collect::<Result<Vec<_>, _>>()?;
            out.color = Some(<[f64; 3]>::try_from(values).map_err(|_| bad())?);
        }
        if let Some(t) = parts.next() {
            out.transparency = t.trim().parse().map_err(|_| bad())?;
        }
        if parts.next().is_some() {
            return Err(bad());
        }
        out.validate()?;
        Ok(out)
    }

    /// The override as [`FilterOverride::parse_spec`] reads it.
    pub fn to_spec(&self) -> String {
        let color = self
            .color
            .map_or_else(|| "-".to_owned(), |[r, g, b]| format!("{r},{g},{b}"));
        format!(
            "{}|{}|{color}|{}",
            self.filter,
            if self.visible { "show" } else { "hide" },
            self.transparency
        )
    }

    /// Admits the override's ranges and filter name; whether the filter exists is the book's
    /// check.
    pub fn validate(&self) -> Result<(), CommandError> {
        validate_label(&self.filter, "views.override.filter")?;
        if !self.transparency.is_finite()
            || !(0.0..=1.0).contains(&self.transparency)
            || self
                .color
                .is_some_and(|c| c.iter().any(|v| !v.is_finite() || !(0.0..=1.0).contains(v)))
        {
            return Err(invalid(
                "views.override",
                "an override's colour and transparency run from 0 to 1",
            ));
        }
        Ok(())
    }
}

/// Admits a view's override list: at most [`MAX_OVERRIDES`], each valid, each filter once.
pub fn validate_overrides(overrides: &[FilterOverride]) -> Result<(), CommandError> {
    if overrides.len() > MAX_OVERRIDES {
        return Err(CommandError::new(
            ErrorCode::LimitExceeded,
            "views.overrides",
            "a view has at most 16 filter overrides",
        ));
    }
    for (at, o) in overrides.iter().enumerate() {
        o.validate()?;
        if overrides
            .iter()
            .take(at)
            .any(|p| p.filter.to_lowercase() == o.filter.to_lowercase())
        {
            return Err(CommandError::new(
                ErrorCode::DuplicateId,
                "views.overrides",
                "a filter is listed once per view",
            ));
        }
    }
    Ok(())
}

/// A named set of display properties a view can be filled from.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewPreset {
    /// Unique name in the book (case-insensitively).
    pub name: String,
    /// Camera projection.
    pub projection: Projection,
    /// Visible height at the view plane.
    pub span_mm: f64,
    /// Visible distance `[above, below]` the plane, `None` for everything.
    pub depth: Option<[f64; 2]>,
    /// Display mode.
    pub render_mode: RenderMode,
    /// Whether grid guides are drawn.
    pub grids_visible: bool,
    /// Element kinds hidden.
    pub hidden_kinds: KindFilter,
    /// Optional colour of the whole model, linear RGB in `0..=1`.
    pub color: Option<[f64; 3]>,
    /// Opacity of the whole model in `0..=1`.
    pub opacity: f64,
    /// The filter overrides, in order.
    pub overrides: Vec<FilterOverride>,
}

impl ViewPreset {
    /// Admits the preset: its name and the same ranges a view admits.
    pub fn validate(&self) -> Result<(), CommandError> {
        validate_label(&self.name, "views.preset.name")?;
        if !self.span_mm.is_finite() || !(MIN_SPAN_MM..=MAX_SPAN_MM).contains(&self.span_mm) {
            return Err(invalid(
                "views.preset.span",
                "the visible height must be between 1 and 20000000 mm",
            ));
        }
        if !self.opacity.is_finite()
            || !(0.0..=1.0).contains(&self.opacity)
            || self
                .color
                .is_some_and(|c| c.iter().any(|v| !v.is_finite() || !(0.0..=1.0).contains(v)))
        {
            return Err(invalid(
                "views.preset.display",
                "colour or opacity is out of range",
            ));
        }
        if let Some([above, below]) = self.depth
            && (!above.is_finite()
                || !below.is_finite()
                || !(0.0..=MAX_DEPTH_MM).contains(&above)
                || !(0.0..=MAX_DEPTH_MM).contains(&below)
                || above + below <= 0.0)
        {
            return Err(invalid(
                "views.preset.depth",
                "view depth needs bounded nonnegative distances and a positive thickness",
            ));
        }
        validate_overrides(&self.overrides)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn conditions_admit_by_field() {
        let ok = |field, op, value: &str| FilterCondition::new(field, op, value).validate();
        assert!(ok(FilterField::Type, Operator::Eq, "beam").is_ok());
        assert!(ok(FilterField::Type, Operator::Eq, "girder").is_err());
        assert!(ok(FilterField::Type, Operator::Lt, "beam").is_err());
        assert!(ok(FilterField::Name, Operator::Contains, "B1").is_ok());
        assert!(ok(FilterField::Class, Operator::Ge, "2").is_ok());
        assert!(ok(FilterField::Class, Operator::Eq, "two").is_err());
        assert!(ok(FilterField::Class, Operator::Contains, "2").is_err());
        assert!(ok(FilterField::Attribute("Phase".into()), Operator::Eq, "1").is_ok());
        assert!(ok(FilterField::Name, Operator::Eq, "   ").is_err());
    }

    #[test]
    fn field_tokens_round_trip() {
        for field in [
            FilterField::Type,
            FilterField::Name,
            FilterField::Class,
            FilterField::Attribute("Fire rating".into()),
        ] {
            assert_eq!(FilterField::parse(&field.token()).unwrap(), field);
        }
        assert!(FilterField::parse("attribute:").is_err());
        assert!(FilterField::parse("colour").is_err());
    }

    #[test]
    fn names_refuse_separators_and_limits() {
        assert!(validate_label("Beams", "f").is_ok());
        assert!(validate_label("A;B", "f").is_err());
        assert!(validate_label(" A", "f").is_err());
        assert!(validate_label(&"x".repeat(65), "f").is_err());
        let rows = vec![FilterCondition::new(FilterField::Type, Operator::Eq, "beam"); 17];
        assert!(ViewFilter::new("Many", rows).validate().is_err());
    }

    #[test]
    fn rows_and_specs_round_trip_as_text() {
        let row = FilterCondition::parse_row("name contains Main beam").unwrap();
        assert_eq!(row.field, FilterField::Name);
        assert_eq!(row.value, "Main beam");
        assert_eq!(FilterCondition::parse_row(&row.to_row()).unwrap(), row);
        let attr = FilterCondition::parse_row("attribute:Fire rating eq R30").unwrap();
        assert_eq!(attr.field, FilterField::Attribute("Fire rating".into()));
        assert!(FilterCondition::parse_row("type beam").is_err());
        assert!(FilterCondition::parse_row("colour eq red").is_err());
        let spec = FilterOverride::parse_spec("Beams|hide|1,0.5,0|0.25").unwrap();
        assert!(!spec.visible);
        assert_eq!(spec.color, Some([1.0, 0.5, 0.0]));
        assert_eq!(FilterOverride::parse_spec(&spec.to_spec()).unwrap(), spec);
        assert!(FilterOverride::parse_spec("Beams").unwrap().visible);
        assert!(FilterOverride::parse_spec("Beams|maybe").is_err());
        assert!(FilterOverride::parse_spec("Beams|show|1,2|0").is_err());
        assert!(FilterOverride::parse_spec("Beams|show|-|0|x").is_err());
    }

    #[test]
    fn overrides_are_listed_once_and_bounded() {
        let a = FilterOverride::new("Beams");
        let mut b = FilterOverride::new("beams");
        assert!(validate_overrides(std::slice::from_ref(&a)).is_ok());
        assert!(validate_overrides(&[a.clone(), b.clone()]).is_err());
        b.filter = "Plates".into();
        b.transparency = 1.5;
        assert!(validate_overrides(&[a, b]).is_err());
    }
}
