//! Compila el CELT de libopus adaptado a SSC (vendor/opus-celt).

fn main() {
    let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor/opus-celt");
    let fuentes = [
        "bands",
        "celt",
        "celt_decoder",
        "celt_encoder",
        "celt_lpc",
        "cwrs",
        "entcode",
        "entdec",
        "entenc",
        "kiss_fft",
        "laplace",
        "mathops",
        "mdct",
        "modes",
        "pitch",
        "quant_bands",
        "rate",
        "vq",
    ];
    let mut cc = cc::Build::new();
    for f in fuentes {
        let ruta = base.join("celt").join(format!("{f}.c"));
        println!("cargo:rerun-if-changed={}", ruta.display());
        cc.file(ruta);
    }
    cc.include(base.join("celt"))
        .include(base.join("include"))
        .include(base.join("silk"))
        .define("OPUS_BUILD", None)
        // Los modos de SSC (864 muestras) no están entre los estáticos de Opus.
        .define("CUSTOM_MODES", None)
        .define("USE_ALLOCA", None)
        .define("HAVE_LRINTF", None)
        .define("HAVE_LRINT", None)
        .define("BASC_SSC", None)
        // El código de libopus no está escrito para estos avisos; no son nuestros.
        .warnings(false)
        .compile("opus_celt_ssc");
    // El directorio entero: un cambio en cualquier cabecera (modes.h,
    // entcode.h…) también tiene que recompilar.
    println!("cargo:rerun-if-changed={}", base.display());
}
