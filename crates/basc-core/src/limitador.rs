//! Ganancia con limitador de picos, antes del encoder.
//!
//! El S22 manda la señal unos 14 dB más fuerte que la fuente (−20 dBFS de
//! entrada salen a −5,7 dBFS en sus tramas, E3) y la controla con su propio
//! limitador/DRC. Los Buds parecen afinados para ese nivel: con la misma
//! fuente, AAC por el PC sonaba «mucho más alto» que BASC (Evelyn, 12-11).
//!
//! Funcionamiento: retrasa una trama (18 ms) para ver el pico de la siguiente
//! antes de procesar la actual. La ganancia de cada trama va en rampa lineal
//! desde la de la trama anterior. Como ambos extremos de la rampa se eligieron
//! sabiendo el pico de la trama actual, ninguna muestra pasa de 1.0, y no hay
//! saltos de ganancia que se oigan como chasquidos. Al bajar el nivel, la
//! ganancia sube despacio (RECUPERACION) para no «bombear».

use crate::codec::{CANALES, MUESTRAS};

const N: usize = MUESTRAS * CANALES;
/// Subida máxima por trama al recuperarse: 0,15 dB/18 ms ≈ 8 dB/s.
const RECUPERACION: f32 = 1.017_419_7; // 10^(0.15/20)

pub struct Limitador {
    ganancia: f32,
    anterior: [f32; N],
    pico_anterior: f32,
    g_anterior: f32,
}

impl Limitador {
    pub fn nuevo(ganancia_db: f32) -> Self {
        Self {
            ganancia: 10f32.powf(ganancia_db / 20.0),
            anterior: [0.0; N],
            pico_anterior: 0.0,
            g_anterior: 1.0,
        }
    }

    /// Procesa una trama y devuelve la anterior, ya con ganancia y limitada a
    /// [-1, 1]. Entrada en [-1, 1] y finita (lo garantiza `Encoder`).
    pub fn procesar(&mut self, x: &[f32; N]) -> [f32; N] {
        let pico = x.iter().fold(0f32, |m, v| m.max(v.abs())) * self.ganancia;
        let techo = |p: f32| if p > 1.0 { 1.0 / p } else { 1.0 };
        let objetivo = techo(self.pico_anterior)
            .min(techo(pico))
            .min(self.g_anterior * RECUPERACION)
            .min(1.0);
        let (desde, paso) = (
            self.g_anterior,
            (objetivo - self.g_anterior) / MUESTRAS as f32,
        );
        let mut salida = [0f32; N];
        for (k, (par, origen)) in salida
            .as_chunks_mut::<CANALES>()
            .0
            .iter_mut()
            .zip(self.anterior.as_chunks::<CANALES>().0)
            .enumerate()
        {
            let g = self.ganancia * (desde + paso * (k + 1) as f32);
            *par = origen.map(|v| (v * g).clamp(-1.0, 1.0));
        }
        self.anterior = *x;
        self.pico_anterior = pico;
        self.g_anterior = objetivo;
        salida
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    fn seno(t: usize, amp: f32) -> [f32; N] {
        let mut x = [0f32; N];
        for (k, par) in x.as_chunks_mut::<2>().0.iter_mut().enumerate() {
            // 500 Hz caben 9 veces exactas en 864 muestras: las tramas empiezan en
            // un paso por cero y cambiar de amplitud entre tramas no mete saltos.
            let v = amp * (2.0 * PI * 500.0 * (t * MUESTRAS + k) as f32 / 48_000.0).sin();
            *par = [v, v];
        }
        x
    }

    #[test]
    fn sin_ganancia_es_un_retraso_de_una_trama() {
        let mut l = Limitador::nuevo(0.0);
        let a = seno(0, 0.5);
        assert_eq!(l.procesar(&a), [0f32; N]);
        assert_eq!(l.procesar(&seno(1, 0.5)), a);
    }

    #[test]
    fn con_12_db_nunca_pasa_de_1_ni_salta() {
        let mut l = Limitador::nuevo(12.0);
        let mut ultima = 0f32;
        let mut salto_max = 0f32;
        // Alterna fuerte y flojo para forzar ataques y recuperaciones.
        for t in 0..200 {
            let amp = if (t / 20) % 2 == 0 { 1.0 } else { 0.05 };
            let y = l.procesar(&seno(t, amp));
            assert!(y.iter().all(|v| v.abs() <= 1.0));
            for v in y.iter().step_by(2) {
                salto_max = salto_max.max((v - ultima).abs());
                ultima = *v;
            }
        }
        // Un seno de 500 Hz a fondo de escala cambia como mucho 2π·500/48000
        // ≈ 0,065 por muestra; un chasquido saltaría mucho más.
        assert!(salto_max < 0.08, "salto de {salto_max}");
    }

    #[test]
    fn la_senal_floja_recibe_la_ganancia_completa() {
        let mut l = Limitador::nuevo(12.0);
        let mut y = [0f32; N];
        for t in 0..10 {
            y = l.procesar(&seno(t, 0.05));
        }
        let pico = y.iter().fold(0f32, |m, v| m.max(v.abs()));
        let esperado = 0.05 * 10f32.powf(12.0 / 20.0);
        assert!(
            (pico - esperado).abs() < 0.01,
            "pico {pico}, esperado {esperado}"
        );
    }
}
