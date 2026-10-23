#import <AppKit/AppKit.h>
#import <Sparkle/Sparkle.h>
#import <Security/Security.h>
#include "sparkle_bridge.h"

static char *json_string(id value) {
    NSData *data = [NSJSONSerialization dataWithJSONObject:value options:0 error:NULL];
    return data ? strdup([[NSString alloc] initWithData:data encoding:NSUTF8StringEncoding].UTF8String) : NULL;
}

static BOOL prototype_bundle(void) {
    return [NSBundle.mainBundle.bundleIdentifier isEqualToString:@"org.hagency.desktop.sparkle-prototype"];
}

static BOOL product_bundle(void) {
    return [NSBundle.mainBundle.bundleIdentifier isEqualToString:@"org.hagency.desktop"] &&
        [NSBundle.mainBundle.infoDictionary[@"HagencySparkleEnabled"] boolValue];
}

char *ha_sparkle_config(void) {
    NSCAssert([NSThread isMainThread], @"Sparkle must stay on the main thread");
    NSBundle *bundle = NSBundle.mainBundle;
    if (!prototype_bundle() && !product_bundle()) return NULL;
    NSDictionary *info = bundle.infoDictionary;
    BOOL valid = NO;
    NSString *team = @"";
    if (product_bundle()) {
        SecCodeRef code = NULL;
        SecRequirementRef requirement = NULL;
        CFDictionaryRef signature = NULL;
        // An ad-hoc or Apple Development signature cannot activate product updates.
        NSString *rule = @"anchor apple generic and certificate 1[field.1.2.840.113635.100.6.2.6] exists and certificate leaf[field.1.2.840.113635.100.6.1.13] exists";
        valid = SecCodeCopySelf(kSecCSDefaultFlags, &code) == errSecSuccess &&
            SecRequirementCreateWithString((__bridge CFStringRef)rule, kSecCSDefaultFlags, &requirement) == errSecSuccess &&
            SecCodeCheckValidity(code, kSecCSStrictValidate, requirement) == errSecSuccess &&
            SecCodeCopySigningInformation(code, kSecCSSigningInformation, &signature) == errSecSuccess;
        if (valid) team = ((__bridge NSDictionary *)signature)[(__bridge NSString *)kSecCodeInfoTeamIdentifier] ?: @"";
        if (signature) CFRelease(signature);
        if (requirement) CFRelease(requirement);
        if (code) CFRelease(code);
    }
    return json_string(@{@"identifier": bundle.bundleIdentifier,
                         @"root": info[@"HagencySparklePrototypeRoot"] ?: @"",
                         @"version": info[@"CFBundleVersion"] ?: @"",
                         @"bundle": bundle.bundlePath,
                         @"feed": info[@"SUFeedURL"] ?: @"",
                         @"enabled": @([info[@"HagencySparkleEnabled"] boolValue]),
                         @"public_key": info[@"SUPublicEDKey"] ?: @"",
                         @"team_id": team, @"developer_id_signed": @(valid),
                         @"signal_only": @((BOOL)(prototype_bundle() && [info[@"HagencySparkleSignalTest"] boolValue])),
                         @"automatic_updates_allowed": info[@"SUAllowsAutomaticUpdates"] ?: NSNull.null,
                         @"automatically_update": info[@"SUAutomaticallyUpdate"] ?: NSNull.null});
}

static NSString *const migration_key = @"HagencySparklePreferencesV1";
static NSString *preferences_domain(void) {
    return NSBundle.mainBundle.infoDictionary[@"SUDefaultsDomain"] ?: NSBundle.mainBundle.bundleIdentifier;
}
static NSUserDefaults *preferences_defaults(void) {
    return [[NSUserDefaults alloc] initWithSuiteName:preferences_domain()];
}
static BOOL migrate_preferences(void) {
    BOOL test = prototype_bundle() && [NSBundle.mainBundle.infoDictionary[@"HagencySparkleTestPreferences"] boolValue];
    return (product_bundle() || test) && ![preferences_defaults() boolForKey:migration_key];
}
bool ha_sparkle_migration_needed(void) {
    NSCAssert([NSThread isMainThread], @"Sparkle must stay on main thread");
    NSDictionary *stored = [preferences_defaults() persistentDomainForName:preferences_domain()];
    return migrate_preferences() && !stored[@"SUEnableAutomaticChecks"];
}

@interface HASparkleBridge : NSObject <SPUUserDriver, SPUUpdaterDelegate>
@property(nonatomic, strong) SPUUpdater *updater;
@property(nonatomic, strong) NSMutableArray<NSDictionary *> *events;
@property(nonatomic, copy) void (^foundReply)(SPUUserUpdateChoice);
@property(nonatomic, copy) void (^readyReply)(SPUUserUpdateChoice);
@property(nonatomic, copy) void (^cancellation)(void);
@property(nonatomic, copy) void (^retryTermination)(void);
@property(nonatomic) uint64_t cycle;
@property(nonatomic) uint64_t received;
@property(nonatomic) uint64_t expected;
@property(nonatomic) HASparkleWake wake;
- (void)emit:(NSString *)kind values:(NSDictionary *)values;
@end

@implementation HASparkleBridge
- (BOOL)updater:(SPUUpdater *)updater mayPerformUpdateCheck:(SPUUpdateCheck)check error:(NSError * __autoreleasing *)error {
    self.cycle += 1; // Automatic and user-initiated sessions use the same cycle fence.
    [self emit:@"checking" values:@{}];
    return YES;
}
- (NSString *)feedURLStringForUpdater:(SPUUpdater *)updater {
    return NSBundle.mainBundle.infoDictionary[@"SUFeedURL"];
}
- (void)updater:(SPUUpdater *)updater didFinishUpdateCycleForUpdateCheck:(SPUUpdateCheck)check error:(NSError *)error {
    // Background no-update checks deliberately do not invoke the user driver's
    // showUpdateNotFound callback. They still have to leave Checking in our UI.
    if ([error.domain isEqualToString:SUSparkleErrorDomain] && error.code == SUNoUpdateError) {
        [self emit:@"current" values:@{}];
    } else if (error) {
        [self emit:@"error" values:@{@"message": error.description}];
    }
    [self emit:@"cycle_finished" values:@{}];
}
- (void)emit:(NSString *)kind values:(NSDictionary *)values {
    NSCAssert([NSThread isMainThread], @"Sparkle callback left main thread");
    NSMutableDictionary *event = [@{@"kind": kind, @"cycle": @(self.cycle)} mutableCopy];
    [event addEntriesFromDictionary:values];
    [self.events addObject:event];
    if (self.wake) self.wake(); // Signal only; never reenter Rust UI objects.
}
- (void)showUpdatePermissionRequest:(SPUUpdatePermissionRequest *)request reply:(void (^)(SUUpdatePermissionResponse *))reply {
    // The harness deliberately disables automatic checking/downloading in Info.plist.
    reply([[SUUpdatePermissionResponse alloc] initWithAutomaticUpdateChecks:NO sendSystemProfile:NO]);
}
- (void)showUserInitiatedUpdateCheckWithCancellation:(void (^)(void))cancellation {
    self.cancellation = cancellation;
    [self emit:@"checking" values:@{}];
}
- (void)showUpdateFoundWithAppcastItem:(SUAppcastItem *)item state:(SPUUserUpdateState *)state reply:(void (^)(SPUUserUpdateChoice))reply {
    self.cancellation = nil;
    if (item.informationOnlyUpdate || state.stage != SPUUserUpdateStageNotDownloaded) {
        reply(state.stage == SPUUserUpdateStageNotDownloaded ? SPUUserUpdateChoiceDismiss : SPUUserUpdateChoiceSkip);
        [self emit:@"error" values:@{@"message": @"Refusing information-only or previously staged updates without admission"}];
        return;
    }
    self.foundReply = reply;
    NSString *notes = item.itemDescription ?: @"";
    if (notes.length > 65536) notes = [notes substringToIndex:65536];
    [self emit:@"available" values:@{@"version": item.displayVersionString ?: @"", @"build": item.versionString ?: @"", @"notes": notes}];
}
- (void)showUpdateReleaseNotesWithDownloadData:(SPUDownloadData *)data {
    if (data.data.length <= 65536 && ([data.MIMEType isEqualToString:@"text/plain"] || [data.MIMEType isEqualToString:@"text/markdown"])) {
        NSString *text = [[NSString alloc] initWithData:data.data encoding:NSUTF8StringEncoding];
        if (text) [self emit:@"notes" values:@{@"text": text}];
    }
}
- (void)showUpdateReleaseNotesFailedToDownloadWithError:(NSError *)error {
    [self emit:@"notes_error" values:@{@"message": error.localizedDescription}];
}
- (void)showUpdateNotFoundWithError:(NSError *)error acknowledgement:(void (^)(void))acknowledgement {
    [self emit:@"current" values:@{}];
    acknowledgement();
}
- (void)showUpdaterError:(NSError *)error acknowledgement:(void (^)(void))acknowledgement {
    [self emit:@"error" values:@{@"message": error.description}];
    acknowledgement();
}
- (void)showDownloadInitiatedWithCancellation:(void (^)(void))cancellation {
    self.received = 0;
    self.expected = 0;
    self.cancellation = cancellation;
    [self emit:@"downloading" values:@{}];
}
- (void)showDownloadDidReceiveExpectedContentLength:(uint64_t)length { self.expected = length; }
- (void)showDownloadDidReceiveDataOfLength:(uint64_t)length {
    self.received += length;
    [self emit:@"progress" values:@{@"received": @(self.received), @"expected": @(self.expected)}];
}
- (void)showDownloadDidStartExtractingUpdate {
    self.cancellation = nil;
    [self emit:@"extracting" values:@{}];
}
- (void)showExtractionReceivedProgress:(double)progress {}
- (void)showReadyToInstallAndRelaunch:(void (^)(SPUUserUpdateChoice))reply {
    self.readyReply = reply;
    [self emit:@"ready" values:@{}];
}
- (void)showInstallingUpdateWithApplicationTerminated:(BOOL)terminated retryTerminatingApplication:(void (^)(void))retry {
    self.retryTermination = terminated ? nil : retry;
    [self emit:@"installing" values:@{@"terminated": @(terminated)}];
}
- (void)showUpdateInstalledAndRelaunched:(BOOL)relaunched acknowledgement:(void (^)(void))acknowledgement {
    [self emit:@"installed" values:@{@"relaunched": @(relaunched)}];
    acknowledgement();
}
- (void)dismissUpdateInstallation {
    self.foundReply = nil;
    self.readyReply = nil;
    self.cancellation = nil;
    self.retryTermination = nil;
    [self emit:@"dismissed" values:@{}];
}
@end

void *ha_sparkle_create(int32_t legacy_auto_check, HASparkleWake wake) {
    NSCAssert([NSThread isMainThread], @"Sparkle must stay on main thread");
    if (!prototype_bundle() && !product_bundle()) return NULL;
    HASparkleBridge *bridge = [HASparkleBridge new];
    bridge.wake = wake;
    bridge.events = [NSMutableArray new];
    bridge.updater = [[SPUUpdater alloc] initWithHostBundle:NSBundle.mainBundle applicationBundle:NSBundle.mainBundle userDriver:bridge delegate:bridge];
    BOOL migrate = migrate_preferences();
    NSUserDefaults *defaults = preferences_defaults();
    if (migrate) {
        // Existing Sparkle choices take precedence over the historical JSON file.
        NSDictionary *stored = [defaults persistentDomainForName:preferences_domain()];
        if (!stored[@"SUEnableAutomaticChecks"]) {
            if (legacy_auto_check != 0 && legacy_auto_check != 1) { bridge.updater = nil; return NULL; }
            bridge.updater.automaticallyChecksForUpdates = legacy_auto_check == 1;
        }
        [defaults setBool:YES forKey:migration_key];
        if (![defaults synchronize]) {
            [defaults removeObjectForKey:migration_key];
            bridge.updater = nil;
            return NULL;
        }
    }
    NSError *error = nil;
    if (![bridge.updater startUpdater:&error]) { bridge.updater = nil; return NULL; }
    [bridge emit:@"preferences" values:@{@"auto_check": @(bridge.updater.automaticallyChecksForUpdates)}];
    return (__bridge_retained void *)bridge;
}

bool ha_sparkle_set_auto_check(void *handle, bool value) {
    NSCAssert([NSThread isMainThread], @"Sparkle must stay on main thread");
    HASparkleBridge *bridge = (__bridge HASparkleBridge *)handle;
    bridge.updater.automaticallyChecksForUpdates = value;
    [bridge emit:@"preferences" values:@{@"auto_check": @(bridge.updater.automaticallyChecksForUpdates)}];
    return [preferences_defaults() synchronize];
}

bool ha_sparkle_can_check(void *handle) {
    NSCAssert([NSThread isMainThread], @"Sparkle must stay on main thread");
    HASparkleBridge *bridge = (__bridge HASparkleBridge *)handle;
    return bridge.updater.canCheckForUpdates && !bridge.updater.sessionInProgress;
}

char *ha_sparkle_next_event(void *handle) {
    NSCAssert([NSThread isMainThread], @"Sparkle must stay on main thread");
    HASparkleBridge *bridge = (__bridge HASparkleBridge *)handle;
    if (bridge.events.count == 0) return NULL;
    NSDictionary *event = bridge.events.firstObject;
    char *result = json_string(event);
    [bridge.events removeObjectAtIndex:0];
    return result;
}

bool ha_sparkle_action(void *handle, uint32_t action, uint64_t cycle) {
    NSCAssert([NSThread isMainThread], @"Sparkle must stay on main thread");
    HASparkleBridge *bridge = (__bridge HASparkleBridge *)handle;
    if (action == 1 && bridge.updater.canCheckForUpdates && !bridge.updater.sessionInProgress) {
        [bridge.updater checkForUpdates];
        return true;
    }
    if (bridge.cycle != cycle) return false;
    if (action == 6 && prototype_bundle()) { // Harness only: AppKit's real QuitRequested path.
        dispatch_async(dispatch_get_main_queue(), ^{ [NSApp terminate:nil]; });
        return true;
    }
    if (action == 2 && bridge.foundReply) {
        void (^reply)(SPUUserUpdateChoice) = bridge.foundReply;
        bridge.foundReply = nil;
        reply(SPUUserUpdateChoiceInstall);
        return true;
    }
    if (action == 3 && bridge.readyReply) {
        void (^reply)(SPUUserUpdateChoice) = bridge.readyReply;
        bridge.readyReply = nil;
        reply(SPUUserUpdateChoiceInstall);
        return true;
    }
    if (action == 4) {
        if (bridge.readyReply) {
            void (^reply)(SPUUserUpdateChoice) = bridge.readyReply;
            bridge.readyReply = nil;
            reply(SPUUserUpdateChoiceSkip); // Dismiss would still install at exit.
        } else if (bridge.foundReply) {
            void (^reply)(SPUUserUpdateChoice) = bridge.foundReply;
            bridge.foundReply = nil;
            reply(SPUUserUpdateChoiceDismiss);
        } else if (bridge.cancellation) {
            void (^cancel)(void) = bridge.cancellation;
            bridge.cancellation = nil;
            cancel();
        } else return false; // Extraction is not cancellable until ready/error.
        return true;
    }
    if (action == 5 && bridge.retryTermination) {
        void (^retry)(void) = bridge.retryTermination;
        bridge.retryTermination = nil;
        retry();
        return true;
    }
    return false;
}
void ha_sparkle_free_string(char *value) { free(value); }
void ha_sparkle_destroy(void *handle) {
    NSCAssert([NSThread isMainThread], @"Sparkle must stay on main thread");
    HASparkleBridge *bridge = (__bridge_transfer HASparkleBridge *)handle;
    // The driver and updater own one another while active. Break this cycle only
    // when the Rust owner is done, never to pretend that an installer was canceled.
    bridge.updater = nil;
}
