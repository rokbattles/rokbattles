/* Read-only descriptor ACL inspection, compiled against the real Apple SDK. */
#include <sys/types.h>
#include <sys/stat.h>
#include <sys/acl.h>
#include <fcntl.h>
#include <errno.h>
#include <stdint.h>
#include <stddef.h>

int rb_capture_acl_protected(int fd) {
    filesec_t security = filesec_init();
    if (!security) return -1;
    struct stat metadata;
    int present = 0;
    acl_t acl = NULL;
    if (fstatx_np(fd, &metadata, security) != 0 ||
        filesec_query_property(security, FILESEC_ACL, &present) != 0) { filesec_free(security); return -1; }
    if (!present) { filesec_free(security); return 0; }
    if (filesec_get_property(security, FILESEC_ACL, &acl) != 0 || !acl) { filesec_free(security); return -1; }
    filesec_free(security);
    if (acl_valid(acl) != 0) { acl_free(acl); return -1; }
    int result = -1;
    for (int index = 0; index <= ACL_MAX_ENTRIES; ++index) {
        acl_entry_t entry = NULL;
        errno = 0;
        if (acl_get_entry(acl, index == 0 ? ACL_FIRST_ENTRY : ACL_NEXT_ENTRY, &entry) != 0) {
            /* Darwin returns EINVAL after the last entry (unlike POSIX Linux). */
            if (errno == EINVAL) result = 0;
            break;
        }
        if (index == ACL_MAX_ENTRIES) break;
        acl_tag_t tag = ACL_UNDEFINED_TAG;
        acl_permset_mask_t permissions = 0;
        if (acl_get_tag_type(entry, &tag) != 0 || acl_get_permset_mask_np(entry, &permissions) != 0) break;
        if (tag == ACL_EXTENDED_DENY) continue;
        if (tag != ACL_EXTENDED_ALLOW) break;
        acl_permset_mask_t safe = ACL_READ_DATA | ACL_EXECUTE | ACL_READ_ATTRIBUTES |
            ACL_READ_EXTATTRIBUTES | ACL_READ_SECURITY | ACL_SYNCHRONIZE;
        /* No principal gets an independent ACL write/delete/ownership grant.
         * Unknown future rights also fail closed; benign read/deny ACLs pass. */
        if (permissions & ~safe) break;
    }
    acl_free(acl);
    return result;
}
