/*!
This module provides three functions that provide 1) device available, 2) device updated, and
3) device leaving notifications over multicast UDP.
*/
use crate::common::httpu::{multicast_once, Options as MulticastOptions, RequestBuilder};
use crate::common::interface::IP;
use crate::common::uri::{URI, URL};
use crate::common::user_agent::user_agent_string;
use crate::discovery::search::SearchTarget;
use crate::discovery::ProductVersion;
use crate::error::{unsupported_version, Error};
use crate::syntax::{
    HTTP_HEADER_BOOTID, HTTP_HEADER_CACHE_CONTROL, HTTP_HEADER_CONFIGID, HTTP_HEADER_HOST, HTTP_HEADER_LOCATION, HTTP_HEADER_NEXT_BOOTID, HTTP_HEADER_NT, HTTP_HEADER_NTS, HTTP_HEADER_SEARCH_PORT, HTTP_HEADER_SERVER, HTTP_HEADER_USN, HTTP_METHOD_NOTIFY, MULTICAST_ADDRESS, MULTICAST_PORT, NTS_ALIVE, NTS_BYE, NTS_UPDATE
};
use crate::SpecVersion;

// ------------------------------------------------------------------------------------------------
// Public Types
// ------------------------------------------------------------------------------------------------

///
/// Description of a device sent in _alive_ and _update_ messages.
///
#[derive(Clone, Debug)]
pub struct Device {
    pub notification_type: SearchTarget,
    pub service_name: URI,
    pub location: URL,
    pub boot_id: u32,
    pub config_id: u64,
    pub search_port: Option<u16>,
    pub secure_location: Option<String>,
}

///
/// This type encapsulates a set of mostly optional values to be used to construct messages to
/// send.
///
#[derive(Clone, Debug)]
pub struct Options {
    /// The specification that will be used to construct sent messages and to verify responses.
    /// Default: `SpecVersion:V10`.
    pub spec_version: SpecVersion,
    /// A specific network interface to bind to; if specified the default address for the interface
    /// will be used, else the address `0.0.0.0:0` will be used. Default: `None`.
    pub network_interface: Option<String>,
    /// Denotes whether the implementation wants to only use IPv4, IPv6, or doesn't care.
    pub network_version: Option<IP>,
    /// The IP packet TTL value.
    pub packet_ttl: u32,
    /// The value used to control caching of these notifications by control points.
    pub max_age: u16,
    /// If specified this is to be the `ProduceName/Version` component of the user agent string
    /// the client will generate as part of sent messages. If not specified a default value based
    /// on the name and version of this crate will be used. Default: `None`.
    pub product_and_version: Option<ProductVersion>,
    /// Multicast address, default: 239.255.255.250
    pub address: Option<String>,
    /// Multicast port, default: 1900
    pub port: Option<u16>,
}

// ------------------------------------------------------------------------------------------------
// Public Functions
// ------------------------------------------------------------------------------------------------

/**
Provides an implementation of the `ssdp:alive` notification.

# Specification

When a device is added to the network, it multicasts discovery messages to advertise its root
device, any embedded devices, and any services. Each discovery message contains four major
components:

1. a potential search target (e.g., device type), sent in an `NT` (Notification Type) header,
2. a composite identifier for the advertisement, sent in a `USN` (Unique Service Name) header,
3. a URL for more information about the device (or enclosing device in the case of a service),
   sent in a `LOCATION` header, and
4. a duration for which the advertisement is valid, sent in a `CACHE-CONTROL` header.

# Parameters

* `device` - details of the device to publish as a part of the notification message. Not all device
     fields may be used in all notifications.
* `options` - protocol options such as the specification version to use and any network
     configuration values.

*/
pub fn device_available(device: &mut Device, options: Options) -> Result<(), Error> {
    let next_boot_id = device.boot_id + 1;
    let mut message_builder = RequestBuilder::new(HTTP_METHOD_NOTIFY);
    message_builder
        .add_header(HTTP_HEADER_HOST, format!("{}:{}", options.address.as_deref().unwrap_or(MULTICAST_ADDRESS), options.port.unwrap_or(MULTICAST_PORT)).as_str())
        .add_header(
            HTTP_HEADER_CACHE_CONTROL,
            &format!("max-age={}", options.max_age),
        )
        .add_header(HTTP_HEADER_LOCATION, &device.location.to_string())
        .add_header(HTTP_HEADER_NT, &device.notification_type.to_string())
        .add_header(HTTP_HEADER_NTS, NTS_ALIVE)
        .add_header(
            HTTP_HEADER_SERVER,
            &user_agent_string(options.spec_version, options.product_and_version.clone()),
        )
        .add_header(HTTP_HEADER_USN, &device.service_name.to_string());

    if options.spec_version >= SpecVersion::V11 {
        message_builder
            .add_header(HTTP_HEADER_BOOTID, &device.boot_id.to_string())
            .add_header(HTTP_HEADER_CONFIGID, &device.config_id.to_string());
        if let Some(search_port) = &device.search_port {
            message_builder.add_header(HTTP_HEADER_SEARCH_PORT, &search_port.to_string());
        }
    }

    if options.spec_version >= SpecVersion::V20 {
        if let Some(secure_location) = &device.secure_location {
            message_builder.add_header(HTTP_HEADER_USN, secure_location);
        }
    }

    multicast_once(
        &message_builder.into(),
        &format!("{}:{}", options.address.as_deref().unwrap_or(MULTICAST_ADDRESS), options.port.unwrap_or(MULTICAST_PORT)).parse().unwrap(),
        &options.into(),
    )?;

    device.boot_id = next_boot_id;
    Ok(())
}

/**
Provides an implementation of the `ssdp:upate` notification.

# Specification

When a new UPnP-enabled interface is added to a multi-homed device, the device MUST increase its
`BOOTID.UPNP.ORG` field value, multicast an `ssdp:update` message for each of the root devices,
embedded devices and embedded services to all of the existing UPnP-enabled interfaces to announce
a change in the `BOOTID.UPNP.ORG` field value, and re-advertise itself on all (existing and new)
UPnP-enabled interfaces with the new `BOOTID.UPNP.ORG` field value. Similarly, if a multi-homed
device loses connectivity on a UPnP-enabled interface and regains connectivity, or if the IP
address on one of the UPnP-enabled interfaces changes, the device MUST increase the
`BOOTID.UPNP.ORG` field value, multicast an `ssdp:update` message for each of the root devices,
embedded devices and embedded services to all the unaffected UPnP-enabled interfaces to announce a
change in the `BOOTID.UPNP.ORG` field value, and re-advertise itself on all (affected and
unaffected) UPnP-enabled interfaces with the new `BOOTID.UPNP.ORG` field value. In all cases, the
`ssdp:update` message for the root devices MUST be sent as soon as possible. Other `ssdp:update`
messages SHOULD be spread over time. However, all ssdp:update messages MUST be sent before any
announcement messages with the new `BOOTID.UPNP.ORG` field value can be sent.


When `ssdp:update` messages are sent on multiple UPnP-enabled interfaces, the messages MUST contain
identical field values except for the `HOST` and `LOCATION` field values. The `HOST` field value
of an advertisement MUST be the standard multicast address specified for the protocol (IPv4 or IPv6)
used on the interface. The URL specified in the `LOCATION` field value MUST be reachable on the
interface on which the advertisement is sent.

# Parameters

* `device` - details of the device to publish as a part of the notification message. Not all device
     fields may be used in all notifications.
* `options` - protocol options such as the specification version to use and any network
     configuration values.

*/
pub fn device_update(device: &mut Device, options: Options) -> Result<(), Error> {
    if options.spec_version == SpecVersion::V10 {
        unsupported_version(options.spec_version).into()
    } else {
        let next_boot_id = device.boot_id + 1;
        let mut message_builder = RequestBuilder::new(HTTP_METHOD_NOTIFY);
        message_builder
            .add_header(HTTP_HEADER_HOST, format!("{}:{}", options.address.as_deref().unwrap_or(MULTICAST_ADDRESS), options.port.unwrap_or(MULTICAST_PORT)).as_str())
            .add_header(HTTP_HEADER_LOCATION, &device.location.to_string())
            .add_header(HTTP_HEADER_NT, &device.notification_type.to_string())
            .add_header(HTTP_HEADER_NTS, NTS_UPDATE)
            .add_header(HTTP_HEADER_USN, &device.service_name.to_string())
            .add_header(HTTP_HEADER_BOOTID, &device.boot_id.to_string())
            .add_header(HTTP_HEADER_NEXT_BOOTID, &next_boot_id.to_string())
            .add_header(HTTP_HEADER_CONFIGID, &device.config_id.to_string());

        if let Some(search_port) = &device.search_port {
            message_builder.add_header(HTTP_HEADER_SEARCH_PORT, &search_port.to_string());
        }

        if options.spec_version >= SpecVersion::V20 {
            if let Some(secure_location) = &device.secure_location {
                message_builder.add_header(HTTP_HEADER_USN, secure_location);
            }
        }

        multicast_once(
            &message_builder.into(),
            &format!("{}:{}", options.address.as_deref().unwrap_or(MULTICAST_ADDRESS), options.port.unwrap_or(MULTICAST_PORT)).parse().unwrap(),
            &options.into(),
        )?;
        device.boot_id = next_boot_id;
        Ok(())
    }
}

/**
Provides an implementation of the `ssdp:byebye` notification.

# Specification

When a device and its services are going to be removed from the network, the device SHOULD
multicast an `ssdp:byebye` message corresponding to each of the `ssdp:alive` messages it multicasted
that have not already expired. If the device is removed abruptly from the network, it might not be
possible to multicast a message. As a fallback, discovery messages MUST include an expiration
value in a `CACHE-CONTROL` field value (as explained above); if not re-advertised, the discovery
message eventually expires on its own.

When a device is about to be removed from the network, it should explicitly revoke its discovery
messages by sending one multicast request for each `ssdp:alive message` it sent. Each multicast
request must have method `NOTIFY` and `ssdp:byeby`e in the `NTS` header in the following format.

# Parameters

* `device` - details of the device to publish as a part of the notification message. Not all device
     fields may be used in all notifications.
* `options` - protocol options such as the specification version to use and any network
     configuration values.

*/
pub fn device_unavailable(device: &mut Device, options: Options) -> Result<(), Error> {
    let next_boot_id = device.boot_id + 1;
    let mut message_builder = RequestBuilder::new(HTTP_METHOD_NOTIFY);
    message_builder
        .add_header(HTTP_HEADER_HOST, format!("{}:{}", options.address.as_deref().unwrap_or(MULTICAST_ADDRESS), options.port.unwrap_or(MULTICAST_PORT)).as_str())
        .add_header(HTTP_HEADER_NT, &device.notification_type.to_string())
        .add_header(HTTP_HEADER_NTS, NTS_BYE)
        .add_header(HTTP_HEADER_USN, &device.service_name.to_string());

    if options.spec_version >= SpecVersion::V11 {
        message_builder
            .add_header(HTTP_HEADER_BOOTID, &device.boot_id.to_string())
            .add_header(HTTP_HEADER_CONFIGID, &device.config_id.to_string());
    }

    multicast_once(
        &message_builder.into(),
        &format!("{}:{}", options.address.as_deref().unwrap_or(MULTICAST_ADDRESS), options.port.unwrap_or(MULTICAST_PORT)).parse().unwrap(),
        &options.into(),
    )?;
    device.boot_id = next_boot_id;
    Ok(())
}

// ------------------------------------------------------------------------------------------------
// Implementations
// ------------------------------------------------------------------------------------------------

const CACHE_CONTROL_MAX_AGE: u16 = 1800;

impl Options {
    pub fn default_for(spec_version: SpecVersion) -> Self {
        Options {
            spec_version,
            network_interface: None,
            network_version: None,
            max_age: CACHE_CONTROL_MAX_AGE,
            packet_ttl: if spec_version == SpecVersion::V10 {
                4
            } else {
                2
            },
            product_and_version: None,
            address: Some(MULTICAST_ADDRESS.to_string()),
            port: Some(MULTICAST_PORT),
            
        }
    }
}

impl From<Options> for MulticastOptions {
    fn from(options: Options) -> Self {
        MulticastOptions {
            network_interface: options.network_interface,
            network_version: options.network_version,
            packet_ttl: options.packet_ttl,
            ..Default::default()
        }
    }
}
