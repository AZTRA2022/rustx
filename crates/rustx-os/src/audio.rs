//! Le son de la machine : quelles sorties et quelles entrées existent,
//! laquelle le système utilise, et le volume général.
//!
//! Ce qu'une page ne peut pas faire. `navigator.mediaDevices` liste les
//! périphériques, mais seulement après que l'utilisateur a donné sa
//! permission au micro, et jamais leur volume : le volume général est
//! au système, pas à l'onglet. Un logiciel qui enregistre, qui appelle
//! ou qui joue de la musique a besoin des deux.
//!
//! # Le volume est celui du périphérique, pas celui de l'application
//!
//! `volume` et `set_volume` lisent et écrivent le volume *du
//! périphérique par défaut* — le même curseur que l'icône de haut-
//! parleur de la barre des tâches. Baisser ce volume baisse le son de
//! toute la machine, et c'est voulu : c'est ce qu'un logiciel de
//! visioconférence ou un lecteur plein écran commande. Le volume de la
//! seule application est une autre chose, qu'aucun des trois systèmes
//! n'expose de la même façon.

use crate::error::OsError;

/// Entrée ou sortie.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flow {
    /// Un haut-parleur, un casque, une sortie HDMI.
    Output,
    /// Un micro, une entrée ligne.
    Input,
}

/// Un périphérique audio.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Endpoint {
    /// Ce que le système en fait sa clé : l'identifiant d'un
    /// `IMMDevice` sous Windows, le nom d'un sink/source PulseAudio
    /// sous Linux, le nom affiché faute de mieux sous macOS.
    pub id: String,
    /// Ce que l'utilisateur lit.
    pub name: String,
    pub flow: Flow,
    /// Celui que le système utilise quand personne n'a choisi.
    pub default: bool,
}

/// Le volume général, de 0 à 1, et si c'est coupé.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Volume {
    pub level: f64,
    pub muted: bool,
}

/// Les périphériques de ce sens-là, celui par défaut en premier.
pub fn endpoints(flow: Flow) -> Result<Vec<Endpoint>, OsError> {
    crate::blocking::waiting("the list of audio devices");
    let mut all = crate::platform::audio::endpoints(flow)?;
    all.sort_by_key(|endpoint| !endpoint.default);
    Ok(all)
}

/// Celui que le système utilise pour ce sens-là.
pub fn default_endpoint(flow: Flow) -> Result<Endpoint, OsError> {
    endpoints(flow)?
        .into_iter()
        .next()
        .ok_or(OsError::NoSuchDevice(match flow {
            Flow::Output => "any audio output".to_string(),
            Flow::Input => "any audio input".to_string(),
        }))
}

/// Le volume du périphérique de sortie par défaut.
pub fn volume() -> Result<Volume, OsError> {
    crate::blocking::waiting("the volume");
    crate::platform::audio::volume()
}

/// Le régler. Hors de `0..=1`, la valeur est ramenée dans l'intervalle
/// — un volume négatif n'est pas une erreur à faire remonter à
/// l'utilisateur, c'est un curseur au bout de sa course.
pub fn set_volume(level: f64) -> Result<(), OsError> {
    crate::blocking::waiting("the volume");
    crate::platform::audio::set_volume(level.clamp(0.0, 1.0))
}

/// Couper ou rendre le son.
pub fn set_muted(muted: bool) -> Result<(), OsError> {
    crate::blocking::waiting("the volume");
    crate::platform::audio::set_muted(muted)
}

// Pris par Linux et macOS ; Windows parle en scalaire de 0 à 1.
#[cfg_attr(windows, allow(dead_code))]
/// Un volume en pour-cent entier, l'unité que Linux et macOS prennent.
pub(crate) fn percent(level: f64) -> u32 {
    (level.clamp(0.0, 1.0) * 100.0).round() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_level_is_a_percentage_inside_its_range() {
        assert_eq!(percent(0.0), 0);
        assert_eq!(percent(0.335), 34);
        assert_eq!(percent(1.0), 100);
        assert_eq!(percent(4.0), 100);
        assert_eq!(percent(-0.5), 0);
    }

    #[test]
    fn this_machine_lists_its_audio_or_refuses_by_name() {
        for flow in [Flow::Output, Flow::Input] {
            match endpoints(flow) {
                Ok(all) => {
                    assert!(
                        all.iter().filter(|endpoint| endpoint.default).count() <= 1,
                        "at most one default per flow: {all:#?}"
                    );
                    for endpoint in &all {
                        assert_eq!(endpoint.flow, flow);
                        assert!(!endpoint.id.is_empty(), "{endpoint:?}");
                    }
                    if let Some(first) = all.first() {
                        assert!(
                            first.default || all.iter().all(|each| !each.default),
                            "the default comes first: {all:#?}"
                        );
                    }
                }
                Err(refused) => assert!(!refused.to_string().is_empty()),
            }
        }
    }
}
