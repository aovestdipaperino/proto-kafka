package dev.protokafka.golden;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ArrayNode;
import com.fasterxml.jackson.databind.node.DoubleNode;
import com.fasterxml.jackson.databind.node.ObjectNode;
import com.fasterxml.jackson.databind.node.TextNode;
import org.apache.kafka.common.Uuid;
import org.apache.kafka.common.compress.Compression;
import org.apache.kafka.common.errors.UnsupportedVersionException;
import org.apache.kafka.common.header.Header;
import org.apache.kafka.common.header.internals.RecordHeader;
import org.apache.kafka.common.message.ApiMessageType;
import org.apache.kafka.common.message.RequestHeaderData;
import org.apache.kafka.common.message.ResponseHeaderData;
import org.apache.kafka.common.protocol.ByteBufferAccessor;
import org.apache.kafka.common.protocol.Message;
import org.apache.kafka.common.protocol.ObjectSerializationCache;
import org.apache.kafka.common.protocol.types.RawTaggedField;
import org.apache.kafka.common.record.BaseRecords;
import org.apache.kafka.common.record.MemoryRecords;
import org.apache.kafka.common.record.Record;
import org.apache.kafka.common.record.RecordBatch;
import org.apache.kafka.common.record.SimpleRecord;
import org.apache.kafka.common.utils.ImplicitLinkedHashCollection;

import java.io.File;
import java.io.FileWriter;
import java.io.IOException;
import java.io.PrintWriter;
import java.lang.reflect.Method;
import java.lang.reflect.ParameterizedType;
import java.lang.reflect.Type;
import java.nio.ByteBuffer;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.Collection;
import java.util.List;
import java.util.Objects;
import java.util.Random;
import java.util.function.Supplier;

/** Emits golden vectors from Kafka's own serializers. Deterministic (fixed seed). */
public final class GoldenGen {
    private static final ObjectMapper M = new ObjectMapper();
    private static final Random R = new Random(0x5EEDL);
    private static final int RANDOM_PER_VERSION = 3;
    private static final int MUTATIONS_PER_VECTOR = 4;
    private static final int MAX_DEPTH = 3;
    private static final Object NO_VALUE = new Object();

    public static void main(String[] args) throws Exception {
        File out = new File(args[0]);
        if (!out.isDirectory() && !out.mkdirs()) throw new IOException("cannot create " + out);
        try (PrintWriter w = new PrintWriter(new FileWriter(new File(out, "messages.jsonl")))) {
            for (ApiMessageType t : ApiMessageType.values()) {
                for (short v = t.lowestSupportedVersion(); v <= t.highestSupportedVersion(true); v++) {
                    emitAll(w, t.apiKey(), "request", v, t::newRequest);
                    emitAll(w, t.apiKey(), "response", v, t::newResponse);
                }
            }
            for (short v = 1; v <= 2; v++) emitAll(w, (short) -1, "request_header", v, RequestHeaderData::new);
            for (short v = 0; v <= 1; v++) emitAll(w, (short) -1, "response_header", v, ResponseHeaderData::new);
        }
        try (PrintWriter w = new PrintWriter(new FileWriter(new File(out, "headers.jsonl")))) {
            for (ApiMessageType t : ApiMessageType.values()) {
                for (short v = t.lowestSupportedVersion(); v <= t.highestSupportedVersion(true); v++) {
                    ObjectNode n = M.createObjectNode();
                    n.put("api_key", t.apiKey());
                    n.put("version", v);
                    n.put("request_header", t.requestHeaderVersion(v));
                    n.put("response_header", t.responseHeaderVersion(v));
                    w.println(M.writeValueAsString(n));
                }
            }
        }
        try (PrintWriter w = new PrintWriter(new FileWriter(new File(out, "records.jsonl")))) {
            emitRecords(w);
        }
    }

    // ---------------------------------------------------------------- messages

    private static void emitAll(PrintWriter w, short key, String kind, short v, Supplier<? extends Message> fresh)
            throws Exception {
        List<byte[]> valid = new ArrayList<>();
        List<String> origins = new ArrayList<>();
        byte[] d = tryWrite(fresh.get(), v);
        if (d != null) { valid.add(d); origins.add("default"); }
        for (int i = 0; i < RANDOM_PER_VERSION; i++) {
            Message m = fresh.get();
            randomize(m, 0);
            byte[] b = writeFittingVersion(m, v);
            if (b != null) { valid.add(b); origins.add("random"); }
        }
        for (int i = 0; i < valid.size(); i++) {
            emitDecoded(w, key, kind, v, fresh, valid.get(i), origins.get(i));
            for (int j = 0; j < MUTATIONS_PER_VECTOR; j++) {
                emitDecoded(w, key, kind, v, fresh, mutate(valid.get(i)), "mutated");
            }
        }
    }

    private static void emitDecoded(PrintWriter w, short key, String kind, short v,
                                    Supplier<? extends Message> fresh, byte[] b, String origin) throws Exception {
        ObjectNode line = M.createObjectNode();
        line.put("api_key", key);
        line.put("kind", kind);
        line.put("version", v);
        line.put("origin", origin);
        line.put("bytes", hex(b));
        ByteBuffer buf = ByteBuffer.wrap(b);
        Message m = fresh.get();
        try {
            m.read(new ByteBufferAccessor(buf), v);
            JsonNode value = toJson(m, v);
            line.put("ok", true);
            line.put("consumed", buf.position());
            line.set("value", value);
            line.putNull("error");
        } catch (RuntimeException e) {
            if (!origin.equals("mutated")) {
                throw new IllegalStateException("Java cannot read its own " + origin + " vector: " + kind + " " + key + " v" + v, e);
            }
            line.put("ok", false);
            line.putNull("consumed");
            line.putNull("value");
            line.put("error", e.toString());
        }
        w.println(M.writeValueAsString(line));
    }

    private static byte[] tryWrite(Message m, short v) {
        try {
            ObjectSerializationCache cache = new ObjectSerializationCache();
            ByteBuffer buf = ByteBuffer.allocate(m.size(cache, v));
            m.write(new ByteBufferAccessor(buf), cache, v);
            return buf.array();
        } catch (RuntimeException e) {
            return null;
        }
    }

    /** Writes at version v, resetting fields Java refuses to write ("Attempted to write a non-default X"). */
    private static byte[] writeFittingVersion(Message m, short v) throws Exception {
        for (int attempt = 0; attempt < 200; attempt++) {
            try {
                ObjectSerializationCache cache = new ObjectSerializationCache();
                ByteBuffer buf = ByteBuffer.allocate(m.size(cache, v));
                m.write(new ByteBufferAccessor(buf), cache, v);
                return buf.array();
            } catch (UnsupportedVersionException e) {
                String msg = String.valueOf(e.getMessage());
                String prefix = "Attempted to write a non-default ";
                int i = msg.indexOf(prefix);
                if (i < 0) return null;
                String field = msg.substring(i + prefix.length()).split(" ")[0];
                if (!resetField(m, field)) return null;
            } catch (RuntimeException e) {
                return null;
            }
        }
        return null;
    }

    private static String cap(String s) {
        return Character.toUpperCase(s.charAt(0)) + s.substring(1);
    }

    /** Zero-arg getters that have a matching one-arg setter: exactly the schema fields.
     *  Excludes next/prev, the ImplicitLinkedHashCollection.Element linkage (not fields). */
    private static List<Method> fieldGetters(Class<?> c) {
        List<Method> out = new ArrayList<>();
        for (Method g : c.getMethods()) {
            if (g.getParameterCount() != 0 || g.getDeclaringClass() != c) continue;
            if (g.getName().equals("next") || g.getName().equals("prev")) continue;
            try {
                c.getMethod("set" + cap(g.getName()), g.getReturnType());
                out.add(g);
            } catch (NoSuchMethodException ignored) {
                // not a field accessor
            }
        }
        return out;
    }

    private static boolean resetField(Object obj, String field) throws Exception {
        boolean changed = false;
        Class<?> c = obj.getClass();
        try {
            Method g = c.getMethod(field);
            Method s = c.getMethod("set" + cap(field), g.getReturnType());
            Object def = g.invoke(c.getDeclaredConstructor().newInstance());
            if (!Objects.equals(g.invoke(obj), def)) {
                s.invoke(obj, def);
                changed = true;
            }
        } catch (NoSuchMethodException ignored) {
            // field not on this struct
        }
        for (Method g : fieldGetters(c)) {
            Object child = g.invoke(obj);
            if (child instanceof Message) changed |= resetField(child, field);
            else if (child instanceof Collection<?> coll) {
                for (Object e : coll) if (e instanceof Message) changed |= resetField(e, field);
            }
        }
        return changed;
    }

    private static void randomize(Object m, int depth) throws Exception {
        for (Method g : fieldGetters(m.getClass())) {
            Method s = m.getClass().getMethod("set" + cap(g.getName()), g.getReturnType());
            Object val = randomValue(s.getParameterTypes()[0], s.getGenericParameterTypes()[0], depth);
            if (val == NO_VALUE) continue;
            try {
                s.invoke(m, val);
            } catch (java.lang.reflect.InvocationTargetException e) {
                // Unsigned fields (e.g. uint16 port held in an int) reject out-of-range values: retry in range.
                if (!(val instanceof Integer)) throw e;
                try {
                    s.invoke(m, R.nextInt(65536));
                } catch (java.lang.reflect.InvocationTargetException e2) {
                    s.invoke(m, R.nextInt(256));
                }
            }
        }
    }

    @SuppressWarnings("unchecked")
    private static Object randomValue(Class<?> c, Type g, int depth) throws Exception {
        if (c == byte.class || c == Byte.class) return (byte) R.nextInt();
        if (c == short.class || c == Short.class) return (short) R.nextInt();
        if (c == int.class || c == Integer.class) return R.nextInt();
        if (c == long.class || c == Long.class) return R.nextLong();
        if (c == double.class || c == Double.class) return R.nextDouble() * 1e6 - 5e5;
        if (c == boolean.class || c == Boolean.class) return R.nextBoolean();
        if (c == String.class) return randomString();
        if (c == byte[].class) return randomBytes();
        if (c == ByteBuffer.class) return ByteBuffer.wrap(randomBytes());
        if (c == Uuid.class) return new Uuid(R.nextLong(), R.nextLong());
        if (BaseRecords.class.isAssignableFrom(c)) return MemoryRecords.EMPTY;
        int n = depth >= MAX_DEPTH ? 0 : R.nextInt(3);
        if (ImplicitLinkedHashCollection.class.isAssignableFrom(c)) {
            Collection<Object> coll = (Collection<Object>) c.getDeclaredConstructor().newInstance();
            Class<?> elem = (Class<?>) ((ParameterizedType) c.getGenericSuperclass()).getActualTypeArguments()[0];
            for (int i = 0; i < n; i++) coll.add(randomValue(elem, elem, depth + 1));
            return coll;
        }
        if (List.class.isAssignableFrom(c)) {
            Type e = ((ParameterizedType) g).getActualTypeArguments()[0];
            Class<?> ec = e instanceof ParameterizedType p ? (Class<?>) p.getRawType() : (Class<?>) e;
            List<Object> l = new ArrayList<>();
            for (int i = 0; i < n; i++) l.add(randomValue(ec, e, depth + 1));
            return l;
        }
        if (Message.class.isAssignableFrom(c)) {
            Object o = c.getDeclaredConstructor().newInstance();
            randomize(o, depth + 1);
            return o;
        }
        return NO_VALUE;
    }

    private static String randomString() {
        String[] parts = {"a", "Z", "0", "-", "é", "日", "😀"};
        StringBuilder sb = new StringBuilder();
        int n = R.nextInt(6);
        for (int i = 0; i < n; i++) sb.append(parts[R.nextInt(parts.length)]);
        return sb.toString();
    }

    private static byte[] randomBytes() {
        byte[] b = new byte[R.nextInt(6)];
        R.nextBytes(b);
        return b;
    }

    private static byte[] mutate(byte[] b) {
        if (b.length == 0) return new byte[] {(byte) 0xff};
        byte[] out;
        switch (R.nextInt(3)) {
            case 0 -> {
                out = new byte[R.nextInt(b.length)];
                System.arraycopy(b, 0, out, 0, out.length);
            }
            case 1 -> {
                out = b.clone();
                out[R.nextInt(b.length)] = (byte) R.nextInt();
            }
            default -> {
                int at = R.nextInt(b.length + 1);
                out = new byte[b.length + 1];
                System.arraycopy(b, 0, out, 0, at);
                out[at] = (byte) 0xff;
                System.arraycopy(b, at, out, at + 1, b.length - at);
            }
        }
        return out;
    }

    /** Java's JsonConverter output, patched: records -> hex bytes, doubles -> raw bits, unknown tags added. */
    private static JsonNode toJson(Message m, short v) throws Exception {
        Class<?> conv = Class.forName(m.getClass().getName() + "JsonConverter");
        Method write = conv.getMethod("write", m.getClass(), short.class, boolean.class);
        JsonNode node = (JsonNode) write.invoke(null, m, v, true);
        patch(m, node);
        return node;
    }

    private static void patch(Object obj, JsonNode node) throws Exception {
        if (!(node instanceof ObjectNode on)) return;
        List<String> keys = new ArrayList<>();
        on.fieldNames().forEachRemaining(keys::add);
        for (String k : keys) {
            Method g;
            try {
                g = obj.getClass().getMethod(k);
            } catch (NoSuchMethodException e) {
                continue;
            }
            Object val = g.invoke(obj);
            JsonNode child = on.get(k);
            if (val instanceof MemoryRecords r) {
                ByteBuffer bb = r.buffer().duplicate();
                byte[] a = new byte[bb.remaining()];
                bb.get(a);
                on.put(k, hex(a));
            } else if (child instanceof DoubleNode && val instanceof Double d) {
                on.set(k, new TextNode(String.format("f64:%016x", Double.doubleToRawLongBits(d))));
            } else if (val instanceof Message) {
                patch(val, child);
            } else if (val instanceof Collection<?> c && child instanceof ArrayNode an) {
                int i = 0;
                for (Object e : c) patch(e, an.get(i++));
            }
        }
        if (obj instanceof Message msg && !msg.unknownTaggedFields().isEmpty()) {
            ArrayNode tags = on.putArray("_unknownTaggedFields");
            for (RawTaggedField f : msg.unknownTaggedFields()) {
                ObjectNode t = tags.addObject();
                t.put("tag", f.tag());
                t.put("data", hex(f.data()));
            }
        }
    }

    // ----------------------------------------------------------------- records

    private static void emitRecords(PrintWriter w) throws Exception {
        List<byte[]> valid = new ArrayList<>();
        for (byte magic : new byte[] {0, 1, 2}) {
            for (int n = 1; n <= 3; n++) {
                for (int rep = 0; rep < 3; rep++) {
                    valid.add(bytes(MemoryRecords.withRecords(magic, R.nextInt(1000), Compression.NONE, randomRecords(n, magic))));
                }
            }
        }
        for (int i = 0; i < 5; i++) {
            valid.add(bytes(MemoryRecords.withIdempotentRecords(RecordBatch.MAGIC_VALUE_V2, R.nextInt(1000), Compression.NONE,
                R.nextLong() & Long.MAX_VALUE, (short) R.nextInt(Short.MAX_VALUE), R.nextInt(Integer.MAX_VALUE),
                R.nextInt(100), randomRecords(1 + R.nextInt(3), (byte) 2))));
            valid.add(bytes(MemoryRecords.withTransactionalRecords(RecordBatch.MAGIC_VALUE_V2, R.nextInt(1000), Compression.NONE,
                R.nextLong() & Long.MAX_VALUE, (short) R.nextInt(Short.MAX_VALUE), R.nextInt(Integer.MAX_VALUE),
                R.nextInt(100), randomRecords(1 + R.nextInt(3), (byte) 2))));
        }
        valid.add(concat(valid.get(0), valid.get(valid.size() - 1)));
        for (byte[] b : valid) {
            emitRecordDecode(w, b, "valid");
            for (int i = 0; i < 6; i++) emitRecordDecode(w, mutate(b), "mutated");
        }
    }

    private static SimpleRecord[] randomRecords(int n, byte magic) {
        SimpleRecord[] out = new SimpleRecord[n];
        for (int i = 0; i < n; i++) {
            long ts = 1_600_000_000_000L + R.nextInt(1_000_000);
            byte[] key = R.nextBoolean() ? null : randomBytes();
            byte[] value = R.nextInt(5) == 0 ? null : randomBytes();
            Header[] headers = new Header[magic == 2 ? R.nextInt(3) : 0];
            for (int h = 0; h < headers.length; h++) {
                headers[h] = new RecordHeader(randomString(), R.nextBoolean() ? null : randomBytes());
            }
            out[i] = new SimpleRecord(ts, key, value, headers);
        }
        return out;
    }

    private static void emitRecordDecode(PrintWriter w, byte[] b, String origin) throws Exception {
        ObjectNode line = M.createObjectNode();
        line.put("origin", origin);
        line.put("bytes", hex(b));
        try {
            ArrayNode batches = M.createArrayNode();
            for (RecordBatch batch : MemoryRecords.readableRecords(ByteBuffer.wrap(b)).batches()) {
                batch.ensureValid();
                ObjectNode bn = batches.addObject();
                bn.put("baseOffset", batch.baseOffset());
                bn.put("lastOffset", batch.lastOffset());
                bn.put("partitionLeaderEpoch", batch.partitionLeaderEpoch());
                bn.put("magic", batch.magic());
                bn.put("crc", batch.checksum());
                bn.put("producerId", batch.producerId());
                bn.put("producerEpoch", batch.producerEpoch());
                bn.put("baseSequence", batch.baseSequence());
                bn.put("isTransactional", batch.isTransactional());
                bn.put("isControl", batch.isControlBatch());
                bn.put("timestampType", batch.timestampType().id);
                bn.put("maxTimestamp", batch.maxTimestamp());
                ArrayNode rs = bn.putArray("records");
                for (Record r : batch) {
                    r.ensureValid();
                    ObjectNode rn = rs.addObject();
                    rn.put("offset", r.offset());
                    rn.put("timestamp", r.timestamp());
                    putHexOrNull(rn, "key", r.hasKey() ? r.key() : null);
                    putHexOrNull(rn, "value", r.hasValue() ? r.value() : null);
                    ArrayNode hs = rn.putArray("headers");
                    for (Header h : r.headers()) {
                        ObjectNode hn = hs.addObject();
                        hn.put("key", hex(h.key().getBytes(StandardCharsets.UTF_8)));
                        if (h.value() == null) hn.putNull("value"); else hn.put("value", hex(h.value()));
                    }
                }
            }
            line.put("ok", true);
            line.set("batches", batches);
            line.putNull("error");
        } catch (RuntimeException e) {
            if (origin.equals("valid")) throw new IllegalStateException("Java cannot read its own record vector", e);
            line.put("ok", false);
            line.putNull("batches");
            line.put("error", e.toString());
        }
        w.println(M.writeValueAsString(line));
    }

    private static void putHexOrNull(ObjectNode n, String k, ByteBuffer b) {
        if (b == null) { n.putNull(k); return; }
        byte[] a = new byte[b.remaining()];
        b.duplicate().get(a);
        n.put(k, hex(a));
    }

    private static byte[] bytes(MemoryRecords r) {
        ByteBuffer bb = r.buffer().duplicate();
        byte[] a = new byte[bb.remaining()];
        bb.get(a);
        return a;
    }

    private static byte[] concat(byte[] a, byte[] b) {
        byte[] out = new byte[a.length + b.length];
        System.arraycopy(a, 0, out, 0, a.length);
        System.arraycopy(b, 0, out, a.length, b.length);
        return out;
    }

    private static String hex(byte[] b) {
        StringBuilder sb = new StringBuilder(b.length * 2);
        for (byte x : b) sb.append(String.format("%02x", x));
        return sb.toString();
    }
}
