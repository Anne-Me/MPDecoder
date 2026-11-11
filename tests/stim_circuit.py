import stim
import pathlib
#print(stim.__version__)


OUT = pathlib.Path("tests/testfiles/")
OUT.mkdir(parents=True, exist_ok=True)
NAME = "surface_code_3_3"

P = 0.01

# use standard surface code example
circuit = stim.Circuit.generated(
    "surface_code:rotated_memory_z",
    distance=3,
    rounds=3,
    after_clifford_depolarization=P,      # noise after gate
    before_round_data_depolarization=P,   # idle noise on data qubits
    before_measure_flip_probability=P,    # measurement errors
    after_reset_flip_probability=P,       # reset errors
)

circuit.to_file(str(OUT / f"{NAME}.stim"))

dem = circuit.detector_error_model()
dem.to_file(str(OUT / f"{NAME}.dem"))
