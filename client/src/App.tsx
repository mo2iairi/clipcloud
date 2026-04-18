import { core } from '@tauri-apps/api'
import { useDeferredValue, useEffect, useMemo, useState } from 'react'
import './App.css'

const { invoke } = core

type Locale = 'zh-CN' | 'en-US'

type SyncState = 'local_only' | 'pending_sync' | 'synced'

type ClipboardHistoryItem = {
  id: string
  content: string
  content_type: string
  sync_state: SyncState
  created_at: string
  last_synced_at: string | null
  origin: string
}

type TrustedDevice = {
  id: string
  device_name: string
  approved_at: string
  revoked_at: string | null
  is_current: boolean
}

type PendingEnrollment = {
  id: string
  device_name: string
  requested_at: string
}

type EnrollmentCode = {
  code: string
  expires_at: string
}

type DeviceProfile = {
  device_id: string
  device_name: string
  capture_mode: string
  server_url: string | null
  is_connected_to_server: boolean
  vault_id: string | null
}

type Snapshot = {
  device: DeviceProfile
  items: ClipboardHistoryItem[]
}

type JoinResponse = {
  enrollment_id: string
  expires_at: string
}

const emptySnapshot: Snapshot = {
  device: {
    device_id: 'pending',
    device_name: '当前设备',
    capture_mode: 'desktop-polling',
    server_url: null,
    is_connected_to_server: false,
    vault_id: null,
  },
  items: [],
}

const translations = {
  'zh-CN': {
    language: '语言',
    chinese: '中文',
    english: 'English',
    never: '从未',
    statusLoading: '正在加载本地剪贴板保险库...',
    statusReady: '本地剪贴板保险库已就绪。',
    statusLoadFailed: '加载应用状态失败。',
    statusActionFailed: '操作失败。',
    heroEyebrow: '私有剪贴板保险库',
    heroTitle: 'ClipCloud',
    heroCopy:
      '以本地优先方式管理剪贴板历史，支持选择性同步、加密保险库传输，以及面向单一自托管拥有者的可信设备接入。',
    metricLocalHistory: '本地历史',
    metricReadyToSync: '待同步',
    metricSynced: '已同步',
    vault: '保险库',
    vaultLocalOnly: '仅本地',
    historyEyebrow: '历史记录',
    historyTitle: '剪贴板时间线',
    historySearch: '搜索本地历史',
    historyLastSynced: '上次同步',
    historySyncButton: '同步到云端',
    historyEmpty: '没有匹配当前筛选条件的剪贴板内容。',
    serverEyebrow: '服务器',
    serverTitle: '初始化或加入',
    serverUrl: '服务器地址',
    deviceName: '设备名称',
    bootstrapButton: '初始化个人服务器',
    joinLabel: '使用注册码加入',
    joinButton: '申请加入',
    activateButton: '激活已批准设备',
    accessEyebrow: '访问控制',
    accessTitle: '可信设备',
    createCodeButton: '生成注册码',
    expiresAt: '过期时间',
    currentDevice: '当前设备',
    revoked: '已撤销',
    revoke: '撤销',
    trustedDevicesEmpty: '初始化服务器后，这里会显示可信设备。',
    reviewEyebrow: '审核',
    reviewTitle: '待批准请求',
    pullHistoryButton: '拉取同步历史',
    pendingEmpty: '当前没有等待批准的设备。',
    approve: '批准',
    syncState: {
      local_only: '仅本地',
      pending_sync: '待同步',
      synced: '已同步',
    },
    statusBootstrapPending: '正在初始化单用户保险库...',
    statusBootstrapSuccess: '服务器初始化完成，当前设备已被信任。',
    statusJoinPending: '正在提交加入请求...',
    statusJoinSuccess: '加入请求已创建。请等待批准，然后在此设备上激活。',
    statusActivatePending: '正在检查是否已批准...',
    statusActivateSuccess: '此设备已激活，现在可以读取已同步历史。',
    statusCodePending: '正在生成短时效注册码...',
    statusCodeSuccess: '注册码已生成。',
    statusRevokePending: '正在撤销可信设备...',
    statusRevokeSuccess: '可信设备已撤销。',
    statusPullPending: '正在下载加密云端历史...',
    statusPullSuccess: '已将同步历史下载并在本地解密。',
    statusApprovePending: '正在为待批准设备封装保险库密钥...',
    statusApproveSuccess: '待批准设备已通过。',
    statusItemSyncPending: '正在加密并同步剪贴板项目...',
    statusItemSyncSuccess: '剪贴板项目已同步到服务器。',
  },
  'en-US': {
    language: 'Language',
    chinese: '中文',
    english: 'English',
    never: 'Never',
    statusLoading: 'Loading local clipboard vault...',
    statusReady: 'Local clipboard vault ready.',
    statusLoadFailed: 'Failed to load application state.',
    statusActionFailed: 'Operation failed.',
    heroEyebrow: 'Private clipboard vault',
    heroTitle: 'ClipCloud',
    heroCopy:
      'Local-first clipboard history with selective sync, encrypted vault transport, and trusted-device enrollment for a single self-hosted owner.',
    metricLocalHistory: 'Local history',
    metricReadyToSync: 'Ready to sync',
    metricSynced: 'Synced',
    vault: 'Vault',
    vaultLocalOnly: 'local-only',
    historyEyebrow: 'History',
    historyTitle: 'Clipboard timeline',
    historySearch: 'Search local history',
    historyLastSynced: 'Last synced',
    historySyncButton: 'Sync to cloud',
    historyEmpty: 'No clipboard items matched that filter.',
    serverEyebrow: 'Server',
    serverTitle: 'Bootstrap or join',
    serverUrl: 'Server URL',
    deviceName: 'Device name',
    bootstrapButton: 'Bootstrap personal server',
    joinLabel: 'Join with enrollment code',
    joinButton: 'Request enrollment',
    activateButton: 'Activate approved device',
    accessEyebrow: 'Access',
    accessTitle: 'Trusted devices',
    createCodeButton: 'Create code',
    expiresAt: 'Expires',
    currentDevice: 'Current device',
    revoked: 'Revoked',
    revoke: 'Revoke',
    trustedDevicesEmpty: 'Trusted devices appear after a server is bootstrapped.',
    reviewEyebrow: 'Review',
    reviewTitle: 'Pending approvals',
    pullHistoryButton: 'Pull synced history',
    pendingEmpty: 'No devices are waiting for approval.',
    approve: 'Approve',
    syncState: {
      local_only: 'local only',
      pending_sync: 'pending sync',
      synced: 'synced',
    },
    statusBootstrapPending: 'Bootstrapping single-owner vault...',
    statusBootstrapSuccess: 'Server bootstrap completed and this device is now trusted.',
    statusJoinPending: 'Submitting enrollment request...',
    statusJoinSuccess:
      'Enrollment request created. Wait for approval, then activate on this device.',
    statusActivatePending: 'Checking for approval...',
    statusActivateSuccess: 'This device has been activated and can now read synced history.',
    statusCodePending: 'Generating short-lived enrollment code...',
    statusCodeSuccess: 'Enrollment code generated.',
    statusRevokePending: 'Revoking trusted device...',
    statusRevokeSuccess: 'Trusted device revoked.',
    statusPullPending: 'Downloading encrypted cloud history...',
    statusPullSuccess: 'Synced history downloaded and decrypted locally.',
    statusApprovePending: 'Wrapping vault key for pending device...',
    statusApproveSuccess: 'Pending device approved.',
    statusItemSyncPending: 'Encrypting and syncing clipboard item...',
    statusItemSyncSuccess: 'Clipboard item synced to the server.',
  },
} as const

function formatTimestamp(value: string | null, locale: Locale) {
  if (!value) {
    return translations[locale].never
  }

  return new Date(value).toLocaleString(locale)
}

function App() {
  const [locale, setLocale] = useState<Locale>('zh-CN')
  const [snapshot, setSnapshot] = useState<Snapshot>(emptySnapshot)
  const [trustedDevices, setTrustedDevices] = useState<TrustedDevice[]>([])
  const [pendingEnrollments, setPendingEnrollments] = useState<PendingEnrollment[]>([])
  const [search, setSearch] = useState('')
  const [status, setStatus] = useState<string>(translations['zh-CN'].statusLoading)
  const [serverUrl, setServerUrl] = useState('http://127.0.0.1:8787')
  const [deviceName, setDeviceName] = useState<string>('当前设备')
  const [joinCode, setJoinCode] = useState('')
  const [enrollmentCode, setEnrollmentCode] = useState<EnrollmentCode | null>(null)
  const [pendingJoin, setPendingJoin] = useState<JoinResponse | null>(null)
  const [isBusy, setIsBusy] = useState(false)
  const deferredSearch = useDeferredValue(search)
  const t = translations[locale]

  async function refreshSnapshot() {
    const next = await invoke<Snapshot>('get_app_snapshot')
    setSnapshot(next)
    setDeviceName(next.device.device_name)
    if (next.device.server_url) {
      setServerUrl(next.device.server_url)
    }
  }

  async function refreshServerState() {
    try {
      const [devices, enrollments] = await Promise.all([
        invoke<TrustedDevice[]>('list_trusted_devices'),
        invoke<PendingEnrollment[]>('list_pending_enrollments'),
      ])
      setTrustedDevices(devices)
      setPendingEnrollments(enrollments)
    } catch {
      setTrustedDevices([])
      setPendingEnrollments([])
    }
  }

  useEffect(() => {
    document.documentElement.lang = locale
  }, [locale])

  useEffect(() => {
    let cancelled = false

    const load = async () => {
      try {
        await refreshSnapshot()
        await refreshServerState()
        if (!cancelled) {
          setStatus(translations[locale].statusReady)
        }
      } catch (error) {
        if (!cancelled) {
          setStatus(
            error instanceof Error ? error.message : translations[locale].statusLoadFailed,
          )
        }
      }
    }

    void load()

    const interval = window.setInterval(() => {
      void refreshSnapshot()
    }, 2200)

    return () => {
      cancelled = true
      window.clearInterval(interval)
    }
  }, [locale])

  const filteredItems = useMemo(() => {
    const needle = deferredSearch.trim().toLowerCase()
    if (!needle) {
      return snapshot.items
    }

    return snapshot.items.filter((item) => item.content.toLowerCase().includes(needle))
  }, [deferredSearch, snapshot.items])

  const localOnlyCount = snapshot.items.filter((item) => item.sync_state === 'local_only').length
  const syncedCount = snapshot.items.filter((item) => item.sync_state === 'synced').length

  async function runAction(action: () => Promise<void>, pendingMessage: string, successMessage: string) {
    setIsBusy(true)
    setStatus(pendingMessage)

    try {
      await action()
      await refreshSnapshot()
      await refreshServerState()
      setStatus(successMessage)
    } catch (error) {
      setStatus(error instanceof Error ? error.message : t.statusActionFailed)
    } finally {
      setIsBusy(false)
    }
  }

  return (
    <div className="shell">
      <header className="hero-panel">
        <div>
          <p className="eyebrow">{t.heroEyebrow}</p>
          <h1>{t.heroTitle}</h1>
          <label className="language-switch">
            <span>{t.language}</span>
            <select value={locale} onChange={(event) => setLocale(event.target.value as Locale)}>
              <option value="zh-CN">{t.chinese}</option>
              <option value="en-US">{t.english}</option>
            </select>
          </label>
          <p className="hero-copy">
            {t.heroCopy}
          </p>
        </div>
        <div className="hero-metrics">
          <article>
            <span>{t.metricLocalHistory}</span>
            <strong>{snapshot.items.length}</strong>
          </article>
          <article>
            <span>{t.metricReadyToSync}</span>
            <strong>{localOnlyCount}</strong>
          </article>
          <article>
            <span>{t.metricSynced}</span>
            <strong>{syncedCount}</strong>
          </article>
        </div>
      </header>

      <main className="layout">
        <section className="panel masthead">
          <div className="device-chip">
            <span>{snapshot.device.device_name}</span>
            <small>{snapshot.device.capture_mode}</small>
          </div>
          <div className="status-line">
            <strong>{status}</strong>
            <span>
              {t.vault}: {snapshot.device.vault_id ? snapshot.device.vault_id.slice(0, 8) : t.vaultLocalOnly}
            </span>
          </div>
        </section>

        <section className="grid">
          <article className="panel history-panel">
            <div className="panel-header">
              <div>
                <p className="eyebrow">{t.historyEyebrow}</p>
                <h2>{t.historyTitle}</h2>
              </div>
              <input
                className="search"
                placeholder={t.historySearch}
                value={search}
                onChange={(event) => setSearch(event.target.value)}
              />
            </div>

            <div className="history-list">
              {filteredItems.map((item) => (
                <article className="history-card" key={item.id}>
                  <div className="history-meta">
                    <span className={`pill pill-${item.sync_state}`}>{t.syncState[item.sync_state]}</span>
                    <span>{formatTimestamp(item.created_at, locale)}</span>
                    <span>{item.origin}</span>
                  </div>
                  <p>{item.content}</p>
                  <div className="history-actions">
                    <small>
                      {t.historyLastSynced}: {formatTimestamp(item.last_synced_at, locale)}
                    </small>
                    {item.sync_state !== 'synced' && (
                      <button
                        disabled={isBusy || !snapshot.device.is_connected_to_server}
                        onClick={() =>
                          void runAction(
                            () => invoke('sync_history_item', { itemId: item.id }),
                            t.statusItemSyncPending,
                            t.statusItemSyncSuccess,
                          )
                        }
                      >
                        {t.historySyncButton}
                      </button>
                    )}
                  </div>
                </article>
              ))}
              {!filteredItems.length && <p className="empty-state">{t.historyEmpty}</p>}
            </div>
          </article>

          <article className="panel action-panel">
            <div className="panel-header tight">
              <div>
                <p className="eyebrow">{t.serverEyebrow}</p>
                <h2>{t.serverTitle}</h2>
              </div>
            </div>

            <label>
              {t.serverUrl}
              <input value={serverUrl} onChange={(event) => setServerUrl(event.target.value)} />
            </label>
            <label>
              {t.deviceName}
              <input value={deviceName} onChange={(event) => setDeviceName(event.target.value)} />
            </label>

            <div className="stack-actions">
              <button
                disabled={isBusy}
                onClick={() =>
                  void runAction(
                    () => invoke('bootstrap_server', { request: { serverUrl, deviceName } }),
                    t.statusBootstrapPending,
                    t.statusBootstrapSuccess,
                  )
                }
              >
                {t.bootstrapButton}
              </button>

              <label>
                {t.joinLabel}
                <input value={joinCode} onChange={(event) => setJoinCode(event.target.value)} />
              </label>
              <button
                disabled={isBusy || !joinCode.trim()}
                onClick={() =>
                  void runAction(
                    async () => {
                      const result = await invoke<JoinResponse>('join_with_enrollment_code', {
                        request: { serverUrl, deviceName, code: joinCode },
                      })
                      setPendingJoin(result)
                    },
                    t.statusJoinPending,
                    t.statusJoinSuccess,
                  )
                }
              >
                {t.joinButton}
              </button>

              {pendingJoin && (
                <button
                  disabled={isBusy}
                  onClick={() =>
                    void runAction(
                      () =>
                        invoke('activate_enrollment', {
                          enrollmentId: pendingJoin.enrollment_id,
                        }),
                      t.statusActivatePending,
                      t.statusActivateSuccess,
                    )
                  }
                >
                  {t.activateButton}
                </button>
              )}
            </div>
          </article>

          <article className="panel action-panel">
            <div className="panel-header tight">
              <div>
                <p className="eyebrow">{t.accessEyebrow}</p>
                <h2>{t.accessTitle}</h2>
              </div>
              <button
                className="secondary"
                disabled={isBusy || !snapshot.device.is_connected_to_server}
                onClick={() =>
                  void runAction(
                    async () => {
                      const code = await invoke<EnrollmentCode>('generate_enrollment_code')
                      setEnrollmentCode(code)
                    },
                    t.statusCodePending,
                    t.statusCodeSuccess,
                  )
                }
              >
                {t.createCodeButton}
              </button>
            </div>

            {enrollmentCode && (
              <div className="callout">
                <strong>{enrollmentCode.code}</strong>
                <span>
                  {t.expiresAt} {formatTimestamp(enrollmentCode.expires_at, locale)}
                </span>
              </div>
            )}

            <div className="list">
              {trustedDevices.map((device) => (
                <div className="list-row" key={device.id}>
                  <div>
                    <strong>{device.device_name}</strong>
                    <small>
                      {device.is_current ? t.currentDevice : formatTimestamp(device.approved_at, locale)}
                    </small>
                  </div>
                  <button
                    className="secondary"
                    disabled={isBusy || device.is_current || Boolean(device.revoked_at)}
                    onClick={() =>
                      void runAction(
                        () => invoke('revoke_trusted_device', { deviceId: device.id }),
                        t.statusRevokePending,
                        t.statusRevokeSuccess,
                      )
                    }
                  >
                    {device.revoked_at ? t.revoked : t.revoke}
                  </button>
                </div>
              ))}
              {!trustedDevices.length && (
                <p className="empty-state">{t.trustedDevicesEmpty}</p>
              )}
            </div>
          </article>

          <article className="panel action-panel">
            <div className="panel-header tight">
              <div>
                <p className="eyebrow">{t.reviewEyebrow}</p>
                <h2>{t.reviewTitle}</h2>
              </div>
              <button
                className="secondary"
                disabled={isBusy || !snapshot.device.is_connected_to_server}
                onClick={() =>
                  void runAction(
                    () => invoke('fetch_synced_history'),
                    t.statusPullPending,
                    t.statusPullSuccess,
                  )
                }
              >
                {t.pullHistoryButton}
              </button>
            </div>
            <div className="list">
              {pendingEnrollments.map((enrollment) => (
                <div className="list-row" key={enrollment.id}>
                  <div>
                    <strong>{enrollment.device_name}</strong>
                    <small>{formatTimestamp(enrollment.requested_at, locale)}</small>
                  </div>
                  <button
                    disabled={isBusy}
                    onClick={() =>
                      void runAction(
                        () => invoke('approve_enrollment', { enrollmentId: enrollment.id }),
                        t.statusApprovePending,
                        t.statusApproveSuccess,
                      )
                    }
                  >
                    {t.approve}
                  </button>
                </div>
              ))}
              {!pendingEnrollments.length && (
                <p className="empty-state">{t.pendingEmpty}</p>
              )}
            </div>
          </article>
        </section>
      </main>
    </div>
  )
}

export default App
