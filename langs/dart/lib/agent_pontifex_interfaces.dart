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

enum AgentKind {
  chatgpt,
  claude,
  codex,
  grok,
  gemini,
  kimi,
  qwen,
  human,
  service,
  other,
}

enum Role { user, assistant, system, tool }
enum MemberRole { owner, member, observer }
enum PresenceKind { joined, left }
enum JobStatus { queued, running, succeeded, failed, cancelled }
enum CompletionOutcome { succeeded, failed }
enum MessageKind {
  command,
  event,
  observation,
  proposal,
  decision,
  handoff,
  heartbeat,
  ack,
  error,
  toolCall,
  toolResult,
  reviewRequest,
  reviewResult,
}

enum DeliveryMode { atMostOnce, atLeastOnce }
enum AckStatus { accepted, rejected, duplicate, expired, unauthorized, staleLease }

AgentKind agentKindFromWire(String value) => AgentKind.values.firstWhere(
      (kind) => kind.name == value,
      orElse: () => AgentKind.other,
    );

String messageKindToWire(MessageKind value) => switch (value) {
      MessageKind.toolCall => 'tool_call',
      MessageKind.toolResult => 'tool_result',
      MessageKind.reviewRequest => 'review_request',
      MessageKind.reviewResult => 'review_result',
      _ => value.name,
    };

MessageKind messageKindFromWire(String value) => switch (value) {
      'tool_call' => MessageKind.toolCall,
      'tool_result' => MessageKind.toolResult,
      'review_request' => MessageKind.reviewRequest,
      'review_result' => MessageKind.reviewResult,
      _ => MessageKind.values.firstWhere((kind) => kind.name == value),
    };

String deliveryModeToWire(DeliveryMode value) => switch (value) {
      DeliveryMode.atMostOnce => 'at_most_once',
      DeliveryMode.atLeastOnce => 'at_least_once',
    };

DeliveryMode deliveryModeFromWire(String value) => switch (value) {
      'at_most_once' => DeliveryMode.atMostOnce,
      'at_least_once' => DeliveryMode.atLeastOnce,
      _ => throw FormatException('unknown delivery mode', value),
    };

String ackStatusToWire(AckStatus value) => switch (value) {
      AckStatus.staleLease => 'stale_lease',
      _ => value.name,
    };

AckStatus ackStatusFromWire(String value) => switch (value) {
      'stale_lease' => AckStatus.staleLease,
      _ => AckStatus.values.firstWhere((status) => status.name == value),
    };

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

class AgentRef {
  const AgentRef({
    required this.agentKey,
    required this.kind,
    required this.instanceId,
    this.displayName,
  });

  final String agentKey;
  final AgentKind kind;
  final String instanceId;
  final String? displayName;

  factory AgentRef.fromJson(Map<String, dynamic> json) => AgentRef(
        agentKey: json['agent_key'] as String,
        kind: agentKindFromWire(json['kind'] as String),
        instanceId: json['instance_id'] as String,
        displayName: json['display_name'] as String?,
      );

  Map<String, dynamic> toJson() => {
        'agent_key': agentKey,
        'kind': kind.name,
        'instance_id': instanceId,
        if (displayName != null) 'display_name': displayName,
      };
}

class TraceContext {
  const TraceContext({
    required this.traceId,
    required this.spanId,
    this.traceFlags,
    this.traceState,
  });

  final String traceId;
  final String spanId;
  final String? traceFlags;
  final String? traceState;

  factory TraceContext.fromJson(Map<String, dynamic> json) => TraceContext(
        traceId: json['trace_id'] as String,
        spanId: json['span_id'] as String,
        traceFlags: json['trace_flags'] as String?,
        traceState: json['trace_state'] as String?,
      );

  Map<String, dynamic> toJson() => {
        'trace_id': traceId,
        'span_id': spanId,
        if (traceFlags != null) 'trace_flags': traceFlags,
        if (traceState != null) 'trace_state': traceState,
      };
}

class LeaseRef {
  const LeaseRef({
    required this.leaseId,
    required this.repository,
    required this.path,
    required this.fencingToken,
    required this.expiresAt,
  });

  final String leaseId;
  final String repository;
  final String path;
  final int fencingToken;
  final Timestamp expiresAt;

  factory LeaseRef.fromJson(Map<String, dynamic> json) => LeaseRef(
        leaseId: json['lease_id'] as String,
        repository: json['repository'] as String,
        path: json['path'] as String,
        fencingToken: json['fencing_token'] as int,
        expiresAt: json['expires_at'] as String,
      );

  Map<String, dynamic> toJson() => {
        'lease_id': leaseId,
        'repository': repository,
        'path': path,
        'fencing_token': fencingToken,
        'expires_at': expiresAt,
      };
}

class RealtimeEnvelope {
  const RealtimeEnvelope({
    required this.messageId,
    required this.conversationId,
    required this.sequence,
    required this.sender,
    required this.recipients,
    required this.kind,
    required this.delivery,
    required this.idempotencyKey,
    required this.payload,
    required this.createdAt,
    this.correlationId,
    this.causationId,
    this.trace,
    this.lease,
    this.expiresAt,
  });

  static const schemaVersion = 'agent-pontifex/realtime-envelope/v1';

  final String messageId;
  final String conversationId;
  final String? correlationId;
  final String? causationId;
  final int sequence;
  final AgentRef sender;
  final List<AgentRef> recipients;
  final MessageKind kind;
  final DeliveryMode delivery;
  final String idempotencyKey;
  final TraceContext? trace;
  final LeaseRef? lease;
  final Map<String, dynamic> payload;
  final Timestamp createdAt;
  final Timestamp? expiresAt;

  factory RealtimeEnvelope.fromJson(Map<String, dynamic> json) =>
      RealtimeEnvelope(
        messageId: json['message_id'] as String,
        conversationId: json['conversation_id'] as String,
        correlationId: json['correlation_id'] as String?,
        causationId: json['causation_id'] as String?,
        sequence: json['sequence'] as int,
        sender: AgentRef.fromJson(json['sender'] as Map<String, dynamic>),
        recipients: (json['recipients'] as List<dynamic>)
            .map((value) => AgentRef.fromJson(value as Map<String, dynamic>))
            .toList(growable: false),
        kind: messageKindFromWire(json['kind'] as String),
        delivery: deliveryModeFromWire(json['delivery'] as String),
        idempotencyKey: json['idempotency_key'] as String,
        trace: json['trace'] == null
            ? null
            : TraceContext.fromJson(json['trace'] as Map<String, dynamic>),
        lease: json['lease'] == null
            ? null
            : LeaseRef.fromJson(json['lease'] as Map<String, dynamic>),
        payload: json['payload'] as Map<String, dynamic>,
        createdAt: json['created_at'] as String,
        expiresAt: json['expires_at'] as String?,
      );

  Map<String, dynamic> toJson() => {
        'schema_version': schemaVersion,
        'message_id': messageId,
        'conversation_id': conversationId,
        if (correlationId != null) 'correlation_id': correlationId,
        if (causationId != null) 'causation_id': causationId,
        'sequence': sequence,
        'sender': sender.toJson(),
        'recipients': recipients.map((value) => value.toJson()).toList(),
        'kind': messageKindToWire(kind),
        'delivery': deliveryModeToWire(delivery),
        'idempotency_key': idempotencyKey,
        if (trace != null) 'trace': trace!.toJson(),
        if (lease != null) 'lease': lease!.toJson(),
        'payload': payload,
        'created_at': createdAt,
        if (expiresAt != null) 'expires_at': expiresAt,
      };
}

class Acknowledgement {
  const Acknowledgement({
    required this.acknowledgementId,
    required this.acknowledgedMessageId,
    required this.conversationId,
    required this.sender,
    required this.status,
    required this.observedSequence,
    required this.createdAt,
    this.reasonCode,
  });

  static const schemaVersion = 'agent-pontifex/acknowledgement/v1';

  final String acknowledgementId;
  final String acknowledgedMessageId;
  final String conversationId;
  final AgentRef sender;
  final AckStatus status;
  final String? reasonCode;
  final int observedSequence;
  final Timestamp createdAt;

  factory Acknowledgement.fromJson(Map<String, dynamic> json) => Acknowledgement(
        acknowledgementId: json['acknowledgement_id'] as String,
        acknowledgedMessageId: json['acknowledged_message_id'] as String,
        conversationId: json['conversation_id'] as String,
        sender: AgentRef.fromJson(json['sender'] as Map<String, dynamic>),
        status: ackStatusFromWire(json['status'] as String),
        reasonCode: json['reason_code'] as String?,
        observedSequence: json['observed_sequence'] as int,
        createdAt: json['created_at'] as String,
      );

  Map<String, dynamic> toJson() => {
        'schema_version': schemaVersion,
        'acknowledgement_id': acknowledgementId,
        'acknowledged_message_id': acknowledgedMessageId,
        'conversation_id': conversationId,
        'sender': sender.toJson(),
        'status': ackStatusToWire(status),
        if (reasonCode != null) 'reason_code': reasonCode,
        'observed_sequence': observedSequence,
        'created_at': createdAt,
      };
}

class WorkHandoff {
  const WorkHandoff({
    required this.handoffId,
    required this.conversationId,
    required this.fromAgent,
    required this.toAgent,
    required this.objective,
    required this.completedWork,
    required this.remainingWork,
    required this.evidenceUris,
    required this.createdAt,
    this.lease,
  });

  static const schemaVersion = 'agent-pontifex/work-handoff/v1';

  final String handoffId;
  final String conversationId;
  final AgentRef fromAgent;
  final AgentRef toAgent;
  final String objective;
  final List<String> completedWork;
  final List<String> remainingWork;
  final List<String> evidenceUris;
  final LeaseRef? lease;
  final Timestamp createdAt;

  factory WorkHandoff.fromJson(Map<String, dynamic> json) => WorkHandoff(
        handoffId: json['handoff_id'] as String,
        conversationId: json['conversation_id'] as String,
        fromAgent: AgentRef.fromJson(json['from_agent'] as Map<String, dynamic>),
        toAgent: AgentRef.fromJson(json['to_agent'] as Map<String, dynamic>),
        objective: json['objective'] as String,
        completedWork: (json['completed_work'] as List<dynamic>).cast<String>(),
        remainingWork: (json['remaining_work'] as List<dynamic>).cast<String>(),
        evidenceUris: (json['evidence_uris'] as List<dynamic>).cast<String>(),
        lease: json['lease'] == null
            ? null
            : LeaseRef.fromJson(json['lease'] as Map<String, dynamic>),
        createdAt: json['created_at'] as String,
      );

  Map<String, dynamic> toJson() => {
        'schema_version': schemaVersion,
        'handoff_id': handoffId,
        'conversation_id': conversationId,
        'from_agent': fromAgent.toJson(),
        'to_agent': toAgent.toJson(),
        'objective': objective,
        'completed_work': completedWork,
        'remaining_work': remainingWork,
        'evidence_uris': evidenceUris,
        if (lease != null) 'lease': lease!.toJson(),
        'created_at': createdAt,
      };
}
