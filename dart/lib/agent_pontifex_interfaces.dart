/// Agent Pontifex interfaces -- Dart projection.
///
/// Types only. No transport, no persistence, no product behaviour. A PROJECTION
/// of the two authorities (schemas/*.json, typespec/main.tsp); do not add fields
/// here that neither authority declares.
library agent_pontifex_interfaces;

const int protocolSchemaVersion = 1;
const String bridgeProtocolId = 'agent-pontifex.bridge';
const String coordinatorProtocolId = 'agent-pontifex.coordinator';
const String discoveryPath = '/.well-known/agent-pontifex';

/// RFC 3339 timestamp. Always a String on the wire, never a number.
typedef Timestamp = String;

enum ServiceKind {
  bridge('bridge', bridgeProtocolId),
  coordinator('coordinator', coordinatorProtocolId);

  const ServiceKind(this.serviceId, this.protocolId);
  final String serviceId;
  final String protocolId;

  static ServiceKind? fromServiceId(String id) {
    for (final k in ServiceKind.values) {
      if (k.serviceId == id) return k;
    }
    return null;
  }
}

enum AgentKind { claude, codex, gemini, kimi, qwen, human, other }
enum Role { user, assistant, system, tool }
enum MemberRole { owner, member, observer }
enum PresenceKind { joined, left }
enum JobStatus { queued, running, succeeded, failed, cancelled }
enum CompletionOutcome { succeeded, failed }

class ProtocolVersionRange {
  const ProtocolVersionRange({required this.minMajor, required this.maxMajor});

  final int minMajor;
  final int maxMajor;

  factory ProtocolVersionRange.fromJson(Map<String, dynamic> json) =>
      ProtocolVersionRange(
        minMajor: json['min_major'] as int,
        maxMajor: json['max_major'] as int,
      );

  Map<String, dynamic> toJson() => {'min_major': minMajor, 'max_major': maxMajor};

  bool get isValid => minMajor >= 1 && minMajor <= maxMajor;

  /// Highest mutually supported protocol major, or null when there is no overlap.
  int? highestCommon(ProtocolVersionRange other) {
    final lower = minMajor > other.minMajor ? minMajor : other.minMajor;
    final upper = maxMajor < other.maxMajor ? maxMajor : other.maxMajor;
    return lower <= upper ? upper : null;
  }
}

class ServiceDescriptor {
  const ServiceDescriptor({
    required this.schemaVersion,
    required this.protocol,
    required this.protocolVersions,
    required this.service,
    required this.implementation,
    this.capabilities = const [],
    this.extensions = const {},
  });

  final int schemaVersion;
  final String protocol;
  final ProtocolVersionRange protocolVersions;
  final String service;
  final String implementation;
  final List<String> capabilities;
  final Map<String, dynamic> extensions;

  factory ServiceDescriptor.fromJson(Map<String, dynamic> json) => ServiceDescriptor(
        schemaVersion: json['schema_version'] as int,
        protocol: json['protocol'] as String,
        protocolVersions: ProtocolVersionRange.fromJson(
            json['protocol_versions'] as Map<String, dynamic>),
        service: json['service'] as String,
        implementation: json['implementation'] as String,
        capabilities:
            (json['capabilities'] as List<dynamic>? ?? const []).cast<String>(),
        extensions:
            (json['extensions'] as Map<String, dynamic>? ?? const {}),
      );

  Map<String, dynamic> toJson() => {
        'schema_version': schemaVersion,
        'protocol': protocol,
        'protocol_versions': protocolVersions.toJson(),
        'service': service,
        'implementation': implementation,
        'capabilities': capabilities,
        if (extensions.isNotEmpty) 'extensions': extensions,
      };

  /// Structural invariants that JSON Schema cannot express.
  /// Returns null when valid, else the first violation.
  String? validate() {
    if (schemaVersion != protocolSchemaVersion) {
      return 'unsupported protocol schema version';
    }
    if (!protocolVersions.isValid) return 'invalid protocol major-version range';
    final kind = ServiceKind.fromServiceId(service);
    if (kind == null) return 'unknown Agent Pontifex service';
    if (protocol != kind.protocolId) {
      return 'service and protocol identifiers do not match';
    }
    final sorted = [...capabilities]..sort();
    for (var i = 0; i < capabilities.length; i++) {
      if (sorted[i] != capabilities[i]) {
        return 'capabilities must be sorted for deterministic negotiation';
      }
      if (!capabilities[i].contains('.')) {
        return 'capability identifiers must use a namespace';
      }
    }
    for (final key in extensions.keys) {
      if (!key.contains('.')) return 'extension keys must use a vendor namespace';
    }
    return null;
  }
}

class FileLease {
  const FileLease({
    required this.id,
    required this.repository,
    required this.path,
    required this.recursive,
    required this.agentKey,
    required this.fencingToken,
    required this.acquiredAt,
    required this.expiresAt,
    this.purpose = '',
  });

  final String id;
  final String repository;
  final String path;
  final bool recursive;
  final String agentKey;

  /// Monotonic per lease line. Compare before any write; never ignore.
  final int fencingToken;
  final Timestamp acquiredAt;
  final Timestamp expiresAt;
  final String purpose;

  factory FileLease.fromJson(Map<String, dynamic> json) => FileLease(
        id: json['id'] as String,
        repository: json['repository'] as String,
        path: json['path'] as String,
        recursive: json['recursive'] as bool,
        agentKey: json['agent_key'] as String,
        fencingToken: json['fencing_token'] as int,
        acquiredAt: json['acquired_at'] as String,
        expiresAt: json['expires_at'] as String,
        purpose: json['purpose'] as String? ?? '',
      );
}
