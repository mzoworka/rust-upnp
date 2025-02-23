/*!
One-line description.

More detailed description, with

# Example

*/

use crate::error::{xml_error, Error};
use crate::syntax::{
    XML_ATTR_NAMESPACE, XML_DECL_VERSION, XML_ELEM_MAJOR, XML_ELEM_MINOR, XML_ELEM_SPEC_VERSION,
};
use crate::SpecVersion;
use quick_xml::events::{attributes::Attribute, BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::Writer;
use std::io::Write;

// ------------------------------------------------------------------------------------------------
// Public Types
// ------------------------------------------------------------------------------------------------

#[derive(Debug)]
pub struct Element {
    name: &'static str,
}

pub trait Writable<T: Write> {
    fn write(&self, writer: &mut Writer<T>) -> Result<(), Error>;
}

pub trait RootWritable<T: Write>: Writable<T> {
    fn write_root(&self, writer: T) -> Result<T, Error> {
        let mut xml = Writer::new(writer);

        start(&mut xml).map_err(xml_error)?;

        self.write(&mut xml)?;

        Ok(xml.into_inner())
    }
}

// ------------------------------------------------------------------------------------------------
// Public Functions
// ------------------------------------------------------------------------------------------------

pub fn start<T: Write>(writer: &mut Writer<T>) -> Result<(), quick_xml::Error> {
    writer
        .write_event(Event::Decl(BytesDecl::new(XML_DECL_VERSION, None, None)))
        .map(|_| ())
        .map_err(|e| quick_xml::Error::Io(e.into()))
}

pub fn element<T: Write>(
    writer: &mut Writer<T>,
    name: &'static str,
) -> Result<(), quick_xml::Error> {
    writer.write_event(Event::Start(BytesStart::new(name)))?;
    writer
        .write_event(Event::End(BytesEnd::new(name)))
        .map(|_| ())
        .map_err(|e| quick_xml::Error::Io(e.into()))
}

pub fn start_element<T: Write>(
    writer: &mut Writer<T>,
    name: &'static str,
) -> Result<Element, quick_xml::Error> {
    writer.write_event(Event::Start(BytesStart::new(name)))
    .map_err(|e| quick_xml::Error::Io(e.into()))?;
    Ok(Element { name })
}

pub fn start_ns_element<T: Write>(
    writer: &mut Writer<T>,
    name: &'static str,
    namespace: &'static str,
    prefix: Option<&str>,
) -> Result<Element, quick_xml::Error> {
    let xmlns = [
        XML_ATTR_NAMESPACE,
        if prefix.is_some() { ":" } else { "" },
        if let Some(p) = prefix { p } else { "" },
    ]
    .concat();

    start_element_with(writer, name, vec![(xmlns.as_str(), namespace)])?;
    Ok(Element { name })
}

pub fn start_element_with<T: Write>(
    writer: &mut Writer<T>,
    name: &'static str,
    attrs: Vec<(&str, &str)>,
) -> Result<Element, quick_xml::Error> {
    let mut element = BytesStart::new(name);
    for (name, value) in attrs {
        element.push_attribute(Attribute::from((name, value)));
    }
    writer.write_event(Event::Start(element))?;
    Ok(Element { name })
}

pub fn end_element<T: Write>(writer: &mut Writer<T>, name: &str) -> Result<(), quick_xml::Error> {
    writer
        .write_event(Event::End(BytesEnd::new(name)))
        .map(|_| ())
        .map_err(|e| quick_xml::Error::Io(e.into()))
}

pub fn text_element<T: Write>(
    writer: &mut Writer<T>,
    name: &'static str,
    content: &str,
) -> Result<(), quick_xml::Error> {
    let element = start_element(writer, name)?;
    writer.write_event(Event::Text(BytesText::new(content)))?;
    element.end(writer)
}

// ------------------------------------------------------------------------------------------------
// Implementations
// ------------------------------------------------------------------------------------------------

impl Element {
    pub fn end<T: Write>(&self, writer: &mut Writer<T>) -> Result<(), quick_xml::Error> {
        end_element(writer, self.name)
    }
}

// ------------------------------------------------------------------------------------------------

// ------------------------------------------------------------------------------------------------

impl<T: Write> Writable<T> for SpecVersion {
    fn write(&self, writer: &mut Writer<T>) -> Result<(), Error> {
        let spec_version = start_element(writer, XML_ELEM_SPEC_VERSION).map_err(xml_error)?;
        text_element(
            writer,
            XML_ELEM_MAJOR,
            match self {
                SpecVersion::V10 => "1",
                SpecVersion::V11 => "1",
                SpecVersion::V20 => "2",
            },
        )
        .map_err(xml_error)?;
        text_element(
            writer,
            XML_ELEM_MINOR,
            match self {
                SpecVersion::V10 => "0",
                SpecVersion::V11 => "1",
                SpecVersion::V20 => "0",
            },
        )
        .map_err(xml_error)?;
        spec_version.end(writer).map_err(xml_error)
    }
}

// ------------------------------------------------------------------------------------------------
// Private Types
// ------------------------------------------------------------------------------------------------

// ------------------------------------------------------------------------------------------------
// Private Functions
// ------------------------------------------------------------------------------------------------

// ------------------------------------------------------------------------------------------------
// Modules
// ------------------------------------------------------------------------------------------------
