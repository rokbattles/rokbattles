/* Read-only libproc bridge. Socket layouts are taken from the installed Apple
 * SDK, never guessed in Rust. No capture, process control, paths or data bytes. */
#include <libproc.h>
#include <sys/proc_info.h>
#include <sys/socket.h>
#include <netinet/in.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <limits.h>

struct rb_process {
    uint64_t start_sec, start_usec;
    uint32_t pid, uid, ruid, svuid, status, flags;
};
struct rb_socket {
    uint64_t generation, socket_id;
    uint8_t local[16], remote[16];
    uint32_t uid, state, shared;
    int32_t fd;
    uint16_t local_port, remote_port;
    uint8_t version;
    uint8_t reserved[3];
};
_Static_assert(sizeof(struct rb_process) == 40, "bridge process ABI");
_Static_assert(sizeof(struct rb_socket) == 72, "bridge socket ABI");

int rb_process_read(int pid, struct rb_process *out) {
    struct proc_bsdinfo info;
    memset(&info, 0, sizeof(info));
    if (proc_pidinfo(pid, PROC_PIDTBSDINFO, 0, &info, sizeof(info)) != (int)sizeof(info)) return -1;
    memset(out, 0, sizeof(*out));
    out->start_sec = info.pbi_start_tvsec;
    out->start_usec = info.pbi_start_tvusec;
    out->pid = info.pbi_pid;
    out->uid = info.pbi_uid;
    out->ruid = info.pbi_ruid;
    out->svuid = info.pbi_svuid;
    out->status = info.pbi_status;
    out->flags = info.pbi_flags;
    return 0;
}
int rb_user_pids(uint32_t uid, int32_t *out, uint32_t capacity) {
    if (capacity == 0 || capacity > 16384) return -1;
    int size = (int)(capacity * sizeof(*out));
    int bytes = proc_listpids(PROC_UID_ONLY, uid, out, size);
    if (bytes <= 0 || bytes >= size || bytes % (int)sizeof(*out) != 0) return -1;
    return bytes / (int)sizeof(*out);
}
int rb_all_pids(int32_t *out, uint32_t capacity) {
    if (capacity == 0 || capacity > 16384) return -1;
    int size = (int)(capacity * sizeof(*out));
    int bytes = proc_listpids(PROC_ALL_PIDS, 0, out, size);
    if (bytes <= 0 || bytes >= size || bytes % (int)sizeof(*out) != 0) return -1;
    return bytes / (int)sizeof(*out);
}
int rb_sockets(int pid, struct rb_socket *out, uint32_t capacity, uint32_t *work) {
    if (capacity == 0 || capacity > 16384) return -1;
    /* One spare row detects a complete buffer rather than silently truncating. */
    int size = (int)((capacity + 1) * sizeof(struct proc_fdinfo));
    struct proc_fdinfo *fds = calloc(capacity + 1, sizeof(*fds));
    if (!fds) return -1;
    int bytes = proc_pidinfo(pid, PROC_PIDLISTFDS, 0, fds, size);
    if (bytes < 0 || bytes >= size || bytes % (int)sizeof(*fds) != 0) { free(fds); return -1; }
    uint32_t visited = (uint32_t)(bytes / (int)sizeof(*fds));
    if (visited > *work) { free(fds); return -1; }
    *work -= visited;
    uint32_t count = 0;
    for (int i = 0; i < bytes / (int)sizeof(*fds); ++i) {
        if (fds[i].proc_fdtype != PROX_FDTYPE_SOCKET) continue;
        struct socket_fdinfo info;
        memset(&info, 0, sizeof(info));
        if (proc_pidfdinfo(pid, fds[i].proc_fd, PROC_PIDFDSOCKETINFO, &info, sizeof(info)) != (int)sizeof(info)) { free(fds); return -1; }
        if (info.psi.soi_kind != SOCKINFO_TCP || info.psi.soi_type != SOCK_STREAM || info.psi.soi_protocol != IPPROTO_TCP) continue;
        int state = info.psi.soi_proto.pri_tcp.tcpsi_state;
        if (state < TSI_S_SYN_SENT || state > TSI_S_TIME_WAIT) continue;
        struct in_sockinfo *inet = &info.psi.soi_proto.pri_tcp.tcpsi_ini;
        if (count >= capacity) { free(fds); return -1; }
        struct rb_socket *row = &out[count];
        memset(row, 0, sizeof(*row));
        if ((info.psi.soi_family == AF_INET || info.psi.soi_family == AF_INET6) && inet->insi_vflag == INI_IPV4) {
            row->version = 4;
            memcpy(row->local + 12, &inet->insi_laddr.ina_46.i46a_addr4, 4);
            memcpy(row->remote + 12, &inet->insi_faddr.ina_46.i46a_addr4, 4);
        } else if (info.psi.soi_family == AF_INET6 && inet->insi_vflag == INI_IPV6) {
            row->version = 6;
            memcpy(row->local, &inet->insi_laddr.ina_6, 16);
            memcpy(row->remote, &inet->insi_faddr.ina_6, 16);
        } else { free(fds); return -1; }
        if (inet->insi_lport < 0 || inet->insi_lport > UINT16_MAX || inet->insi_fport < 0 || inet->insi_fport > UINT16_MAX) { free(fds); return -1; }
        row->local_port = ntohs((uint16_t)inet->insi_lport);
        row->remote_port = ntohs((uint16_t)inet->insi_fport);
        row->generation = inet->insi_gencnt;
        row->socket_id = info.psi.soi_so;
        row->uid = info.psi.soi_stat.vst_uid;
        row->state = info.psi.soi_proto.pri_tcp.tcpsi_state;
        row->shared = (info.pfi.fi_status & PROC_FP_SHARED) != 0;
        row->fd = fds[i].proc_fd;
        ++count;
    }
    free(fds);
    return (int)count;
}
