/* Read-only installed/running-agent validation using Apple's Security framework.
 * Signing identifiers are fixed release contracts. No shell or credentials. */
#include <CoreFoundation/CoreFoundation.h>
#include <Security/Security.h>
#include <libproc.h>
#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>
#include <string.h>

static int hardened(CFDictionaryRef information) {
    CFTypeRef value = CFDictionaryGetValue(information, kSecCodeInfoFlags);
    uint32_t flags = 0;
    if (!value || CFGetTypeID(value) != CFNumberGetTypeID() ||
        !CFNumberGetValue((CFNumberRef)value, kCFNumberSInt32Type, &flags) ||
        !(flags & kSecCodeSignatureRuntime) || (flags & kSecCodeSignatureAdhoc)) return 0;
    CFTypeRef entitlements = CFDictionaryGetValue(information, kSecCodeInfoEntitlementsDict);
    if (entitlements) {
        if (CFGetTypeID(entitlements) != CFDictionaryGetTypeID()) return 0;
        CFStringRef forbidden[] = {
            CFSTR("com.apple.security.get-task-allow"),
            CFSTR("com.apple.security.cs.disable-library-validation"),
            CFSTR("com.apple.security.cs.allow-dyld-environment-variables"),
            CFSTR("com.apple.security.cs.allow-unsigned-executable-memory"),
            CFSTR("com.apple.security.cs.disable-executable-page-protection"),
            CFSTR("com.apple.security.cs.allow-jit")
        };
        for (unsigned i = 0; i < sizeof(forbidden) / sizeof(forbidden[0]); ++i) {
            value = CFDictionaryGetValue((CFDictionaryRef)entitlements, forbidden[i]);
            if (value && !CFEqual(value, kCFBooleanFalse)) return 0;
        }
    }
    return 1;
}
int rb_agent_valid(int pid) {
    int result = -1;
    char path[PROC_PIDPATHINFO_MAXSIZE] = {0};
    const char *expected = "/Library/Application Support/ROK Battles/rokbattles-desktop-agent";
    if (pid <= 0 || proc_pidpath(pid, path, sizeof(path)) <= 0 || strcmp(path, expected) != 0) return -1;
    SecCodeRef helper = NULL, peer = NULL;
    SecStaticCodeRef installed = NULL;
    SecRequirementRef anchor = NULL, requirement = NULL;
    CFDictionaryRef helper_info = NULL, peer_info = NULL, installed_info = NULL, attributes = NULL;
    CFNumberRef process = NULL;
    CFStringRef expression = NULL;
    CFURLRef url = NULL;
    CFTypeRef team = NULL, peer_hash = NULL, installed_hash = NULL;
    const void *keys[1] = { kSecGuestAttributePid }, *values[1] = { NULL };
    if (SecCodeCopySelf(kSecCSDefaultFlags, &helper) != errSecSuccess ||
        SecRequirementCreateWithString(CFSTR("anchor apple generic and identifier \"com.rokbattles.capture-helper\""), kSecCSDefaultFlags, &anchor) != errSecSuccess ||
        SecCodeCheckValidity(helper, kSecCSDefaultFlags, anchor) != errSecSuccess ||
        SecCodeCopySigningInformation((SecStaticCodeRef)helper, kSecCSSigningInformation, &helper_info) != errSecSuccess ||
        !hardened(helper_info)) goto cleanup;
    team = CFDictionaryGetValue(helper_info, kSecCodeInfoTeamIdentifier);
    if (!team || CFGetTypeID(team) != CFStringGetTypeID() || CFStringGetLength((CFStringRef)team) != 10) goto cleanup;
    /* Validate before interpolation; the authenticated helper supplies the team. */
    for (CFIndex i = 0; i < 10; ++i) {
        UniChar ch = CFStringGetCharacterAtIndex((CFStringRef)team, i);
        if (!((ch >= 'A' && ch <= 'Z') || (ch >= '0' && ch <= '9'))) goto cleanup;
    }
    expression = CFStringCreateWithFormat(NULL, NULL, CFSTR("anchor apple generic and identifier \"com.rokbattles.desktop-agent\" and certificate leaf[subject.OU] = \"%@\""), team);
    if (!expression || SecRequirementCreateWithString(expression, kSecCSDefaultFlags, &requirement) != errSecSuccess) goto cleanup;
    url = CFURLCreateFromFileSystemRepresentation(NULL, (const UInt8 *)expected, (CFIndex)strlen(expected), false);
    if (!url || SecStaticCodeCreateWithPath(url, kSecCSDefaultFlags, &installed) != errSecSuccess ||
        SecStaticCodeCheckValidity(installed, kSecCSStrictValidate | kSecCSCheckAllArchitectures, requirement) != errSecSuccess ||
        SecCodeCopySigningInformation(installed, kSecCSSigningInformation, &installed_info) != errSecSuccess || !hardened(installed_info)) goto cleanup;
    process = CFNumberCreate(NULL, kCFNumberIntType, &pid);
    if (!process) goto cleanup;
    values[0] = process;
    attributes = CFDictionaryCreate(NULL, keys, values, 1, &kCFTypeDictionaryKeyCallBacks, &kCFTypeDictionaryValueCallBacks);
    if (!attributes || SecCodeCopyGuestWithAttributes(NULL, attributes, kSecCSDefaultFlags, &peer) != errSecSuccess ||
        SecCodeCheckValidity(peer, kSecCSDefaultFlags, requirement) != errSecSuccess ||
        SecCodeCopySigningInformation((SecStaticCodeRef)peer, kSecCSSigningInformation | kSecCSDynamicInformation, &peer_info) != errSecSuccess || !hardened(peer_info)) goto cleanup;
    CFTypeRef status_value = CFDictionaryGetValue(peer_info, kSecCodeInfoStatus);
    uint32_t status = 0;
    if (!status_value || CFGetTypeID(status_value) != CFNumberGetTypeID() ||
        !CFNumberGetValue((CFNumberRef)status_value, kCFNumberSInt32Type, &status) ||
        !(status & kSecCodeStatusValid) || (status & kSecCodeStatusDebugged)) goto cleanup;
    peer_hash = CFDictionaryGetValue(peer_info, kSecCodeInfoUnique);
    installed_hash = CFDictionaryGetValue(installed_info, kSecCodeInfoUnique);
    if (!peer_hash || !installed_hash || CFGetTypeID(peer_hash) != CFDataGetTypeID() ||
        CFGetTypeID(installed_hash) != CFDataGetTypeID() || !CFEqual(peer_hash, installed_hash)) goto cleanup;
    result = 0;
cleanup:
    if (attributes) CFRelease(attributes);
    if (process) CFRelease(process);
    if (url) CFRelease(url);
    if (expression) CFRelease(expression);
    if (requirement) CFRelease(requirement);
    if (anchor) CFRelease(anchor);
    if (installed_info) CFRelease(installed_info);
    if (peer_info) CFRelease(peer_info);
    if (helper_info) CFRelease(helper_info);
    if (installed) CFRelease(installed);
    if (peer) CFRelease(peer);
    if (helper) CFRelease(helper);
    return result;
}
