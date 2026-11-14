//! Le son, sous Windows : Core Audio (`IMMDeviceEnumerator`,
//! `IAudioEndpointVolume`).
//!
//! C'est l'API que le panneau de son et l'icône de la barre des tâches
//! utilisent eux-mêmes : le volume lu ici est exactement celui que
//! l'utilisateur voit bouger.
//!
//! # Deux volumes, et celui qu'on prend
//!
//! `GetMasterVolumeLevelScalar` et non `GetMasterVolumeLevel` : le
//! second est en décibels, une échelle où « la moitié » ne veut rien
//! dire à un curseur. Le scalaire est de 0 à 1 et suit la courbe que
//! Windows applique au curseur, donc 0,5 ici est le curseur au milieu.

use windows::Win32::Devices::FunctionDiscovery::PKEY_Device_FriendlyName;
use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
use windows::Win32::Media::Audio::{
    DEVICE_STATE_ACTIVE, EDataFlow, IMMDevice, IMMDeviceEnumerator, MMDeviceEnumerator, eCapture,
    eConsole, eRender,
};
use windows::Win32::System::Com::{
    CLSCTX_ALL, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
    CoTaskMemFree, STGM_READ,
};
use windows::Win32::System::Com::StructuredStorage::PropVariantToStringAlloc;

use crate::audio::{Endpoint, Flow, Volume};
use crate::error::OsError;

fn flowing(flow: Flow) -> EDataFlow {
    match flow {
        Flow::Output => eRender,
        Flow::Input => eCapture,
    }
}

fn refused(what: &str, error: windows::core::Error) -> OsError {
    OsError::Platform(format!("{what} was refused by Core Audio: {error}"))
}

fn enumerator() -> Result<IMMDeviceEnumerator, OsError> {
    // SAFETY: COM pour ce fil ; déjà initialisé, l'appel le dit et c'est
    // sans conséquence.
    let _ = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    // SAFETY: la classe documentée de Core Audio.
    unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_INPROC_SERVER) }
        .map_err(|error| refused("the audio device list", error))
}

/// L'identifiant d'un périphérique, et `CoTaskMemFree` derrière :
/// `GetId` rend une chaîne que l'appelant possède.
fn id_of(device: &IMMDevice) -> Result<String, OsError> {
    // SAFETY: l'interface est vivante ; la chaîne rendue est à nous et
    // libérée ici.
    unsafe {
        let wide = device.GetId().map_err(|error| refused("a device id", error))?;
        let text = wide.to_string().unwrap_or_default();
        CoTaskMemFree(Some(wide.as_ptr().cast()));
        Ok(text)
    }
}

/// Le nom que l'utilisateur lit, ou l'identifiant faute de mieux : un
/// périphérique sans nom affichable reste un périphérique utilisable.
fn name_of(device: &IMMDevice, fallback: &str) -> String {
    // SAFETY: le magasin de propriétés du périphérique, ouvert en
    // lecture ; la chaîne rendue par `PropVariantToStringAlloc` est à
    // nous et libérée ici.
    unsafe {
        let Ok(store) = device.OpenPropertyStore(STGM_READ) else {
            return fallback.to_string();
        };
        let Ok(value) = store.GetValue(&PKEY_Device_FriendlyName) else {
            return fallback.to_string();
        };
        match PropVariantToStringAlloc(&value) {
            Ok(wide) => {
                let text = wide.to_string().unwrap_or_default();
                CoTaskMemFree(Some(wide.as_ptr().cast()));
                if text.is_empty() {
                    fallback.to_string()
                } else {
                    text
                }
            }
            Err(_) => fallback.to_string(),
        }
    }
}

pub fn endpoints(flow: Flow) -> Result<Vec<Endpoint>, OsError> {
    let enumerator = enumerator()?;
    let direction = flowing(flow);
    // Le périphérique par défaut peut ne pas exister : une machine sans
    // carte son en a zéro, et ce n'est pas une panne.
    // SAFETY: l'énumérateur est vivant.
    let default = unsafe { enumerator.GetDefaultAudioEndpoint(direction, eConsole) }
        .ok()
        .and_then(|device| id_of(&device).ok());
    // SAFETY: idem ; seuls les périphériques actifs sont demandés — un
    // casque débranché est présent dans Windows et inutilisable.
    let collection = unsafe { enumerator.EnumAudioEndpoints(direction, DEVICE_STATE_ACTIVE) }
        .map_err(|error| refused("the audio device list", error))?;
    // SAFETY: idem.
    let count = unsafe { collection.GetCount() }
        .map_err(|error| refused("the audio device count", error))?;
    let mut found = Vec::with_capacity(count as usize);
    for index in 0..count {
        // SAFETY: un index que `GetCount` vient de borner.
        //
        // Les parenthèses ne sont pas décoratives : un `let … else` dont
        // l'expression se termine par un bloc est refusé par le
        // compilateur, et c'est le cas de tout `unsafe { … }`.
        let Ok(device) = (unsafe { collection.Item(index) }) else {
            continue;
        };
        let Ok(id) = id_of(&device) else { continue };
        let name = name_of(&device, &id);
        found.push(Endpoint {
            default: default.as_deref() == Some(id.as_str()),
            id,
            name,
            flow,
        });
    }
    Ok(found)
}

/// Le contrôle de volume du périphérique de sortie par défaut.
fn master() -> Result<IAudioEndpointVolume, OsError> {
    let enumerator = enumerator()?;
    // SAFETY: l'énumérateur est vivant.
    let device = unsafe { enumerator.GetDefaultAudioEndpoint(eRender, eConsole) }
        .map_err(|error| refused("the default audio output", error))?;
    // SAFETY: le périphérique est vivant ; `Activate` rend l'interface
    // demandée par son type.
    unsafe { device.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None) }
        .map_err(|error| refused("the volume control", error))
}

pub fn volume() -> Result<Volume, OsError> {
    let master = master()?;
    // SAFETY: l'interface est vivante ; deux lectures.
    unsafe {
        let level = master
            .GetMasterVolumeLevelScalar()
            .map_err(|error| refused("reading the volume", error))?;
        let muted = master
            .GetMute()
            .map_err(|error| refused("reading the mute state", error))?;
        Ok(Volume {
            level: f64::from(level),
            muted: muted.as_bool(),
        })
    }
}

pub fn set_volume(level: f64) -> Result<(), OsError> {
    let master = master()?;
    // SAFETY: l'interface est vivante ; le contexte d'événement nul dit
    // « ce changement ne vient pas d'un de mes propres rappels ».
    unsafe { master.SetMasterVolumeLevelScalar(level as f32, std::ptr::null()) }
        .map_err(|error| refused("setting the volume", error))
}

pub fn set_muted(muted: bool) -> Result<(), OsError> {
    let master = master()?;
    // SAFETY: idem.
    unsafe { master.SetMute(muted, std::ptr::null()) }
        .map_err(|error| refused("muting", error))
}
