// SPDX-FileCopyrightText: Copyright 2025 Dmitry Marakasov <amdmi3@amdmi3.ru>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::HashMap;

use bitflags::bitflags;

use repology_common::{LinkType, PackageFlags};

use crate::package::{ExtraField, Link, ParsedPackage};
use crate::parsing::error::PackageParsingError;
use crate::parsing::utils::version::VersionStripper;

#[derive(Debug, Clone, Default)]
pub struct PackageMaker {
    subrepo: Option<String>,

    srcname: Option<String>,
    binname: Option<String>,
    binnames: Vec<String>,
    trackname: Option<String>,
    visiblename: Option<String>,
    projectname_seed: Option<String>,

    rawversion: Option<String>,

    arch: Option<String>,

    maintainers: Vec<String>,
    category: Option<String>,
    comment: Option<String>,
    licenses: Vec<String>,

    extrafields: HashMap<String, ExtraField>,

    cpe_vendor: Option<String>,
    cpe_product: Option<String>,
    cpe_edition: Option<String>,
    cpe_lang: Option<String>,
    cpe_sw_edition: Option<String>,
    cpe_target_sw: Option<String>,
    cpe_target_hw: Option<String>,
    cpe_other: Option<String>,

    links: Vec<Link>,

    version: Option<String>,

    flags: PackageFlags,
    flavors: Vec<String>,
}

bitflags! {
    #[derive(Default, Debug, PartialEq, Clone, Copy, Eq)]
    pub struct NameType: u32 {
        const SrcName         = 1 << 0;
        const BinName         = 1 << 1;
        const TrackName       = 1 << 2;
        const DisplayName     = 1 << 3;
        const ProjectNameSeed = 1 << 4;
    }
}

impl PackageMaker {
    pub fn set_names(&mut self, name: impl Into<String>, name_types: NameType) -> &mut Self {
        // TODO: strip, forbid newlines
        let name = name.into();
        if name_types.contains(NameType::SrcName) {
            if self.srcname.is_some() {
                panic!("SrcName set twice");
            }
            self.srcname = Some(name.clone());
        }
        if name_types.contains(NameType::BinName) {
            if self.binname.is_some() {
                panic!("BinName set twice");
            }
            self.binname = Some(name.clone());
        }
        if name_types.contains(NameType::TrackName) {
            if self.trackname.is_some() {
                panic!("TrackName set twice");
            }
            self.trackname = Some(name.clone());
        }
        if name_types.contains(NameType::DisplayName) {
            if self.visiblename.is_some() {
                panic!("DisplayName set twice");
            }
            self.visiblename = Some(name.clone());
        }
        if name_types.contains(NameType::ProjectNameSeed) {
            if self.projectname_seed.is_some() {
                panic!("ProjectNameSeed set twice");
            }
            self.projectname_seed = Some(name);
        }
        self
    }

    pub fn add_binnames(
        &mut self,
        binnames: impl IntoIterator<Item = impl Into<String>>,
    ) -> &mut Self {
        binnames
            .into_iter()
            .map(|binname| binname.into())
            .collect_into(&mut self.binnames);
        self
    }

    pub fn set_version(&mut self, version: impl Into<String>) -> &mut Self {
        // TODO: strip, forbid newlines
        let version = version.into();
        self.rawversion = Some(version.clone());
        self.version = Some(version);
        self
    }

    pub fn set_version_stripped(
        &mut self,
        version: impl Into<String>,
        stripper: &VersionStripper,
    ) -> &mut Self {
        // TODO: strip, forbid newlines
        let version = version.into();
        let stripped = stripper.apply(&version).to_string();
        self.rawversion = Some(version);
        self.version = Some(stripped);
        self
    }

    pub fn set_summary(&mut self, summary: impl Into<String>) -> &mut Self {
        // TODO: strip, limit length
        self.comment = Some(summary.into());
        self
    }

    pub fn add_maintainer(&mut self, maintainer: impl Into<String>) -> &mut Self {
        // TODO: strip, forbid newlines, lowercase, unicalize
        self.maintainers.push(maintainer.into());
        self
    }

    pub fn add_maintainers(
        &mut self,
        maintainers: impl IntoIterator<Item = impl Into<String>>,
    ) -> &mut Self {
        maintainers.into_iter().for_each(|maintainer| {
            self.add_maintainer(maintainer);
        });
        self
    }

    pub fn add_category(&mut self, category: impl Into<String>) -> &mut Self {
        // TODO: allow multiple categories
        // TODO: strip, forbid newlines
        if self.category.is_none() {
            self.category = Some(category.into());
        }
        self
    }

    pub fn add_categories(
        &mut self,
        categories: impl IntoIterator<Item = impl Into<String>>,
    ) -> &mut Self {
        categories.into_iter().for_each(|category| {
            self.add_category(category);
        });
        self
    }

    pub fn add_link(&mut self, link_type: LinkType, url: impl Into<String>) -> &mut Self {
        let url: String = url.into();
        if let Some((url, fragment)) = url.split_once('#') {
            self.links.push(Link {
                r#type: link_type,
                url: url.to_owned(),
                fragment: Some(fragment.to_owned()),
            });
        } else {
            self.links.push(Link {
                r#type: link_type,
                url,
                fragment: None,
            });
        }
        self
    }

    pub fn add_links(
        &mut self,
        link_type: LinkType,
        urls: impl IntoIterator<Item = impl Into<String>>,
    ) -> &mut Self {
        urls.into_iter().for_each(|url| {
            self.add_link(link_type, url);
        });
        self
    }

    pub fn set_extra_field_one(&mut self, name: &str, value: impl Into<String>) {
        self.extrafields
            .insert(name.to_string(), ExtraField::OneValue(value.into()));
    }

    pub fn set_extra_field_many(
        &mut self,
        name: &str,
        values: impl IntoIterator<Item = impl Into<String>>,
    ) {
        let vec: Vec<_> = values.into_iter().map(|v| v.into()).collect();
        if !vec.is_empty() {
            self.extrafields
                .insert(name.to_string(), ExtraField::ManyValues(vec));
        }
    }

    pub fn finalize(self) -> Result<ParsedPackage, PackageParsingError> {
        let projectname_seed = self
            .projectname_seed
            .ok_or(PackageParsingError::MissingProjectNameSeed)?;
        if projectname_seed.is_empty() {
            return Err(PackageParsingError::EmptyProjectNameSeed);
        }

        let visiblename = self
            .visiblename
            .ok_or(PackageParsingError::MissingVisibleName)?;
        if visiblename.is_empty() {
            return Err(PackageParsingError::EmptyVisibleName);
        }

        let version = self.version.ok_or(PackageParsingError::MissingVersion)?;
        if version.is_empty() {
            return Err(PackageParsingError::EmptyVersion);
        }

        if self.srcname.is_none() && self.binname.is_none() && self.binnames.is_empty() {
            return Err(PackageParsingError::MissingPackageNames);
        }
        if self.srcname.as_ref().is_some_and(|name| name.is_empty()) {
            return Err(PackageParsingError::EmptySrcName);
        }
        if self.binname.as_ref().is_some_and(|name| name.is_empty())
            || self.binnames.iter().any(|name| name.is_empty())
        {
            return Err(PackageParsingError::EmptyBinName);
        }

        Ok(ParsedPackage {
            subrepo: self.subrepo,

            srcname: self.srcname,
            binname: self.binname,
            binnames: self.binnames, //.map(|binnames| binnames.into_iter().unique().collect()),
            trackname: self.trackname,
            visiblename,
            projectname_seed: projectname_seed.clone(),

            rawversion: self
                .rawversion
                .expect("rawversion is expected to be set as long as version is set"),

            arch: self.arch,

            maintainers: self.maintainers,
            category: self.category,
            comment: self.comment,
            licenses: self.licenses,

            extrafields: self.extrafields,

            cpe_vendor: self.cpe_vendor,
            cpe_product: self.cpe_product,
            cpe_edition: self.cpe_edition,
            cpe_lang: self.cpe_lang,
            cpe_sw_edition: self.cpe_sw_edition,
            cpe_target_sw: self.cpe_target_sw,
            cpe_target_hw: self.cpe_target_hw,
            cpe_other: self.cpe_other,

            links: self.links,

            version,

            flags: self.flags,
            flavors: self.flavors,
        })
    }
}

#[cfg(test)]
#[coverage(off)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn test_panic_double_srcname() {
        let mut pkg = PackageMaker::default();
        pkg.set_names("foo", NameType::SrcName);
        pkg.set_names("foo", NameType::SrcName);
    }

    #[test]
    #[should_panic]
    fn test_panic_double_binname() {
        let mut pkg = PackageMaker::default();
        pkg.set_names("foo", NameType::BinName);
        pkg.set_names("foo", NameType::BinName);
    }

    #[test]
    #[should_panic]
    fn test_panic_double_trackname() {
        let mut pkg = PackageMaker::default();
        pkg.set_names("foo", NameType::TrackName);
        pkg.set_names("foo", NameType::TrackName);
    }

    #[test]
    #[should_panic]
    fn test_panic_double_displayname() {
        let mut pkg = PackageMaker::default();
        pkg.set_names("foo", NameType::DisplayName);
        pkg.set_names("foo", NameType::DisplayName);
    }

    #[test]
    #[should_panic]
    fn test_panic_double_projectname_seed() {
        let mut pkg = PackageMaker::default();
        pkg.set_names("foo", NameType::ProjectNameSeed);
        pkg.set_names("foo", NameType::ProjectNameSeed);
    }

    #[test]
    fn test_simple() {
        let mut pkg = PackageMaker::default();
        pkg.set_names("bin", NameType::BinName);
        pkg.set_names("src", NameType::SrcName);
        pkg.set_names("track", NameType::TrackName);
        pkg.set_names("display", NameType::DisplayName);
        pkg.set_names("project", NameType::ProjectNameSeed);
        pkg.set_version("1.2.3");
        let package = pkg.finalize().unwrap();

        assert_eq!(package.binname, Some("bin".to_string()));
        assert_eq!(package.srcname, Some("src".to_string()));
        assert_eq!(package.trackname, Some("track".to_string()));
        assert_eq!(package.visiblename, "display".to_string());
        assert_eq!(package.projectname_seed, "project".to_string());
        assert_eq!(package.version, "1.2.3".to_string());
        assert_eq!(package.rawversion, "1.2.3".to_string());
    }

    fn finalize_test_package(mut pkg: PackageMaker) -> ParsedPackage {
        // set mandatory fields
        // XXX: provide in PackageMaker API and use here a way to
        // check whether the field has been set before
        pkg.set_names("foobar", NameType::all());
        pkg.set_version("1.2.3");
        pkg.finalize().unwrap()
    }

    #[test]
    fn test_set_extra_field_one() {
        let mut pkg = PackageMaker::default();
        pkg.set_extra_field_one("foo", "bar1");
        pkg.set_extra_field_one("foo", "bar2");
        let package = finalize_test_package(pkg);
        assert_eq!(
            package.extrafields["foo"],
            ExtraField::OneValue("bar2".to_string())
        );
    }

    #[test]
    fn test_set_extra_field_many() {
        let mut pkg = PackageMaker::default();
        pkg.set_extra_field_many("foo", ["bar1", "bar1"]);
        pkg.set_extra_field_many("foo", ["bar3", "bar4"]);
        let package = finalize_test_package(pkg);
        assert_eq!(
            package.extrafields["foo"],
            ExtraField::ManyValues(vec!["bar3".to_string(), "bar4".to_string()])
        );
    }

    #[test]
    fn test_add_link_no_framgent() {
        let mut pkg = PackageMaker::default();
        pkg.add_link(LinkType::UpstreamHomepage, "https://example.com/");
        let package = finalize_test_package(pkg);
        assert_eq!(
            package.links,
            vec![Link {
                r#type: LinkType::UpstreamHomepage,
                url: "https://example.com/".to_string(),
                fragment: None,
            }],
        );
    }

    #[test]
    fn test_add_link_framgent() {
        let mut pkg = PackageMaker::default();
        pkg.add_link(LinkType::UpstreamHomepage, "https://example.com/foo#frag");
        let package = finalize_test_package(pkg);
        assert_eq!(
            package.links,
            vec![Link {
                r#type: LinkType::UpstreamHomepage,
                url: "https://example.com/foo".to_string(),
                fragment: Some("frag".to_string()),
            }],
        );
    }
}
