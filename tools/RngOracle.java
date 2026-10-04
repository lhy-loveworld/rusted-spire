import com.badlogic.gdx.math.RandomXS128;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.Random;

/** Test driver only: the RNG implementation comes from the user's local source.
 * Wrapper calls mirror cardcrawl.random.Random; the game wrapper is not compiled.
 */
public final class RngOracle {
    private static void row(long seed, String op, long a, long b, String value, int counter) {
        System.out.println(Long.toUnsignedString(seed) + "\t" + op + "\t"
            + a + "\t" + b + "\t" + value + "\t" + counter);
    }

    private static long undoRight(long value, int shift) {
        long result = value;
        for (int i = 0; i < 64; i += shift) result = value ^ (result >>> shift);
        return result;
    }

    private static long undoLeft(long value, int shift) {
        long result = value;
        for (int i = 0; i < 64; i += shift) result = value ^ (result << shift);
        return result;
    }

    public static void main(String[] args) {
        long[] seeds = {0, 1, 42, -1, Long.MIN_VALUE, Long.MAX_VALUE, 123456789};
        for (long seed : seeds) {
            RandomXS128 rng = new RandomXS128(seed);
            int counter = 0;
            for (int i = 0; i < 16; i++) {
                for (int range : new int[] {0, 1, 99, Integer.MAX_VALUE - 1}) {
                    row(seed, "int", range, 0, Integer.toString(rng.nextInt(range + 1)), ++counter);
                }
                row(seed, "range", -5, 12, Integer.toString(-5 + rng.nextInt(18)), ++counter);
                row(seed, "bool", 0, 0, rng.nextBoolean() ? "1" : "0", ++counter);
                for (float chance : new float[] {0.0f, 0.4f, 1.0f}) {
                    row(seed, "chance", Float.floatToRawIntBits(chance), 0,
                        rng.nextFloat() < chance ? "1" : "0", ++counter);
                }
                row(seed, "float", 0, 0, Integer.toUnsignedString(Float.floatToRawIntBits(rng.nextFloat())), ++counter);
                row(seed, "long", 0, 0, Long.toUnsignedString(rng.nextLong()), ++counter);
            }
            rng = new RandomXS128(seed);
            counter = 0;
            for (int size : new int[] {0, 1, 2, 5, 10, 17, 100}) {
                List<Integer> cards = new ArrayList<>();
                for (int i = 0; i < size; i++) cards.add(i);
                Collections.shuffle(cards, new Random(rng.nextLong()));
                Collections.reverse(cards); // Java top-at-end -> Rust top-at-zero
                String value = String.join(",", cards.stream().map(Object::toString).toList());
                row(seed, "shuffle", size, 0, value, ++counter);
            }
        }
        // Force the first raw output to -1: nextInt(100) must reject it.
        long state0 = undoLeft(undoRight(-1L, 17), 23);
        RandomXS128 rejection = new RandomXS128(state0, 0);
        if (new RandomXS128(state0, 0).nextLong() != -1L) throw new AssertionError();
        row(state0, "forced_int", 0, 99, Integer.toString(rejection.nextInt(100)), 1);
        row(state0, "long", 0, 0, Long.toUnsignedString(rejection.nextLong()), 2);
    }
}
