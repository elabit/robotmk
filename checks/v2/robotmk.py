#!/usr/bin/python
# -*- encoding: utf-8; py-indent-offset: 4 -*-

# (c) 2021 Simon Meggle <simon.meggle@elabit.de>

# This file is part of Robotmk
# https://robotmk.org
# https://github.com/simonmeggle/robotmk

# Robotmk is free software;  you can redistribute it and/or modify it
# under the  terms of the  GNU General Public License  as published by
# the Free Software Foundation in version 3.  This file is distributed
# in the hope that it will be useful, but WITHOUT ANY WARRANTY;  with-
# out even the implied warranty of  MERCHANTABILITY  or  FITNESS FOR A
# PARTICULAR PURPOSE. See the  GNU General Public License for more de-
# ails.  You should have  received  a copy of the  GNU  General Public
# License along with GNU Make; see the file  COPYING.  If  not,  write
# to the Free Software Foundation, Inc., 51 Franklin St,  Fifth Floor,
# Boston, MA 02110-1301 USA.

import os
import json, base64, zlib
import re
import time, datetime
from dateutil.tz import tzlocal
from dateutil import parser
import xml.etree.ElementTree as ET
from string import Template
from random import randint
import shutil
from collections import namedtuple

# V2 specific
# from .agent_based_api.v1 import *
from cmk.base.plugins.agent_based.agent_based_api.v1 import *
from cmk.utils.exceptions import MKGeneralException

ROBOTMK_VERSION = 'v1.2-beta.3'
DEFAULT_SVC_PREFIX = 'Robot Framework E2E $SUITEID$SPACE-$SPACE'
HTML_LOG_DIR = "%s/%s" % (os.environ['OMD_ROOT'], 'local/share/addons/robotmk')

STATES = {
    0: 'OK',
    1: 'WARNING',
    2: 'CRITICAL',
    3: 'UNKNOWN',
}
STATES_NO = {
    'OK': 0,
    'WARNING': 1,
    'CRITICAL': 2,
    'UNKNOWN': 3
}
ROBOT_NAGIOS_STATUS = {'PASS': 0, 'FAIL': 2}

STATE_BADGES = {0: '', 1: '(!)', 2: '(!!)', 3: 'UNKNOWN'}

ROBOTMK_KEYWORDS = {
    'Add Checkmk Test State',
    'Add Monitoring Message',
}

def parse_robotmk(params, string_table):
    keys_to_decode = ['xml', 'htmllog']
    robot_discovery_settings = params.get('robot_discovery_settings', [])
    try:
        st_joined = ''.join([l[0] for l in string_table])
        st_dict = json.loads(st_joined)
    except Exception:
        raise MKGeneralException(
            "Can not load Robotmk JSON data! (json.loads())")

    runner_data = st_dict['runner']
    for idx, json_suite in enumerate(st_dict['suites']):
        for k in keys_to_decode:
            if k in json_suite:
                if bool(json_suite[k]):
                    d = json_suite[k]
                    if runner_data['encoding'] == 'zlib_codec':
                        d = d.encode('utf-8')
                        d_byte = base64.b64decode(d)
                        d_decomp = zlib.decompress(d_byte).decode('utf-8')
                    elif runner_data['encoding'] == 'base64_codec':
                        d = d.encode('utf-8')
                        d_decomp = base64.b64decode(d)
                    else:
                        d_decomp = d
                    json_suite[k] = d_decomp
        try:
            xml = ET.fromstring(json_suite['xml'])
            xml_root_suite = xml.find('./suite')
        except Exception:
            continue
            # Seems to be a good idea not to raise an exception here.
            # The Robotmk service can report this error, too.
            #raise MKGeneralException("Fatal parsing error. Robotmk cannot " +\
            #    "find XML/HTML data. %s" % suite.get('error', ''))
        setting = pattern_match(robot_discovery_settings,
                                xml_root_suite.attrib['name'], (0, ''))
        discovery_setting = namedtuple(
            'DiscoverySetting', 'level blacklist_pattern')._make(setting)
        # now process the root suite
        st_dict['suites'][idx]['parsed'] = parse_suite_xml(
            xml_root_suite, discovery_setting)
    return (st_dict, params.__dict__['_data'])


# v2discovery
def discover_robotmk(params, section):
    info_dict, params_dict = parse_robotmk(params, section)
    service_prefix = params.get('robot_service_prefix', [])
    html_show_patterns = params_dict.get('htmllog', {})
    is_piggyback_result = info_dict['runner'].get('is_piggyback_result', False)
    svc_label_robotmk_yes = ServiceLabel(u"robotmk", u"yes")
    for root_suite in info_dict['suites']:
        suite_piggybackhost = root_suite.get('piggybackhost', "")
        if 'parsed' in root_suite:
            for discovered_item in root_suite['parsed'].discovered:
                service_description = add_svc_prefix(discovered_item.name,
                                               root_suite,
                                               service_prefix)
                svc_labels_htmllog = []
                if (not is_piggyback_result and not suite_piggybackhost) or (is_piggyback_result and suite_piggybackhost): 
                    # Displaying a hyperlink to the HTML logs on CMK services 
                    # See Ref #VfHCJn in robotmk agent plugin
                    svc_labels_htmllog = assign_html_logs(service_description, info_dict, root_suite, html_show_patterns)
                    svc_labels = svc_labels_htmllog + [svc_label_robotmk_yes]
                    yield Service(
                        item=service_description,
                        parameters=params_dict,
                        labels=svc_labels)
 
    # Display the Robotmk meta service only on the "Robot" host. (for reporting overall runtimes, stale spool files etc.)
    if not is_piggyback_result: 
        svc_robotmk = params.get('robotmk_service_name', 'Robotmk')
        svc_labels = [ServiceLabel(u"robotmk", u"yes"), ServiceLabel(u"robotmk/type", u"robotmk")]
        yield Service(
            item=svc_robotmk,
            parameters=params_dict, 
            labels=svc_labels)


def check_robotmk(item, params, section):
    parsed_section, params_dict = parse_robotmk(params, section)
    service_prefix = params.get('robot_service_prefix', [])
    svc_robotmk = params_dict.get('robotmk_service_name', 'Robotmk')
    runner_assigned_host = parsed_section['runner'].get('assigned_host', [])
    # The "Robotmk" service
    if item == svc_robotmk:
        # item is the Robotmk meta service
        perfdata_list = []
        suites_total = len(parsed_section['suites'])
        rc = 0
        # list of strings for first output line
        first_line = []
        # lines 2ff.
        out_lines = []

        # I) Staleness Check:
        # result_age vs. cache_time/execution time

        suites_fatal = check_fatal_suites(parsed_section['suites'])
        suites_stale, suites_nonstale = check_stale_suites(parsed_section['suites'])
        # firstline
        if suites_total == 0:
            first_line.append(
                "0 suites planned/executed (!). Check the configuration!")
            rc = max(rc, 1)
        else:
            if len(suites_nonstale) > 0:
                first_line.append(
                    "%d of %s suite(s) have recent results (%s)" %
                    (len(suites_nonstale), suites_total,
                     quoted_listitems([suite.id for suite in suites_nonstale])))
            if len(suites_stale) > 0:
                # Ref: N2UC9N
                # Stale results are only alarmed by the Robotmk service. The Robotmk
                # service itself gets only stale. 
                rc = max(rc, 2)
                first_line.append(
                    "stale suites: %s (!!) (%s)" %
                    (len(suites_stale),
                     quoted_listitems([suite.id for suite in suites_stale])))
                out_lines.extend([suite.msg for suite in suites_stale])
            if len(suites_fatal) > 0:
                rc = max(rc, 2)
                first_line.append("FATAL suites: %s (!!) (%s)" %
                                  (len(suites_fatal), ', '.join([
                                      "Suite '%s': %s" % (s['id'], s['error'])
                                      for s in suites_fatal
                                  ])))

        yield Metric("suites_total", suites_total)
        yield Metric('suites_nonstale', len(suites_nonstale))
        yield Metric('suites_stale', len(suites_stale))
        yield Metric('suites_fatal', len(suites_fatal))

        # II) Headroom monitoring
        '''A non-selective (=complete) run is whenever the runner gets started 
        with no suite args and all suites are run as configured. 
        In this case the runtime headroom should be monitored:
        - serial mode (controller itself starts runner with no suite args)
        - external mode (when a scheduled task starts the runner with no suite args)
        A selective, non-complete run is 
        - parallel mode (controller starts one runner per suite)
        - external mode (a scheduled task starts the runner with suite args)'''

        if 'runtime_total' in parsed_section['runner']:
            runner_runtime = round(parsed_section['runner']["runtime_total"], 1)

            try:
                if parsed_section['runner']['execution_mode'] == 'agent_serial':
                    cache_time = parsed_section['runner']['cache_time']
                    execution_interval = parsed_section['runner']['execution_interval']
                    maxruntime = execution_interval
                    maxruntime_str = 'execution interval'
                elif parsed_section['runner'][
                        'execution_mode'] == 'external' and not parsed_section['runner']['selective_run']:
                    cache_time = parsed_section['runner']['cache_time']
                    maxruntime = cache_time
                    maxruntime_str = 'cache time'
                    execution_interval = None
            finally:
                # FIXME: This are w/c threshold PLACEHOLDERS !!
                runner_runtime_warn_s = maxruntime * 0.9
                runner_runtime_crit_s = maxruntime * 0.95
                pct_runtime_usage = round(
                    (100 / float(maxruntime)) * runner_runtime, 1)
                if runner_runtime > runner_runtime_warn_s:
                    if runner_runtime > runner_runtime_crit_s:
                        badge = '(!!) '
                        rc = max(rc, 2)
                    else:
                        badge = '(!) '
                        rc = max(rc, 1)
                else:
                    badge = ''
                    rc = max(rc, 0)
                first_line.append(
                    "%slast runner execution used %.1f%% (%.1fs) of " %
                    (badge, pct_runtime_usage, runner_runtime) + "%s (%ds)" %
                    (maxruntime_str, maxruntime))

                # TODO: Add _real_ warn/crit thresholds here
                yield Metric(
                    "runner_runtime", 
                    runner_runtime, 
                    levels=(runner_runtime_warn_s, runner_runtime_warn_s), 
                    boundaries=(0, maxruntime)
                )
                yield Metric(
                    "runner_cache_time", 
                    cache_time, 
                )
                if not execution_interval is None:
                    yield Metric(
                        "runner_execution_interval", 
                        execution_interval, 
                    )                    
                yield Metric(
                    "runner_runtime_robotmk", 
                    float("%.3f" % parsed_section['runner']["runtime_robotmk"]), 
                )
                yield Metric(
                    "runner_runtime_suites", 
                    float("%.3f" % parsed_section['runner']["runtime_suites"]), 
                )
        else:
            rc = max(rc, 2)
            first_line.append("Robotmk Runner did never run (!!)")

        # 3. Execution mode
        first_line.append("execution mode: %s" %
                          parsed_section['runner']["execution_mode"])
        
        # 4. Check Robotmk messages (coming from keyword: "Add Monitoring Message")
        # see Ref. 8nIZ5J
        fflines = []
        for root_suite in parsed_section['suites']:
            if len(root_suite['parsed'].robotmk_messages) > 0: 
                firstline_messages = []
                fflines.append("Messages from suite '%s':" % root_suite['parsed'].name)
                suite_rc = 0
                for data in root_suite['parsed'].robotmk_messages:
                    stateid = STATES_NO[data['nagios_state']]
                    badge = STATE_BADGES[stateid]
                    fflines.append(" %s %s %s" % (u"\u25cf", badge, data['msg']))
                    rc = max(rc, stateid)
                    suite_rc = max(suite_rc, stateid)
                first_line.append("Suite '%s' has messages %s" % (
                    root_suite['parsed'].name,
                    STATE_BADGES[suite_rc],
                    ))
        out_lines.append('\n'.join(fflines))


        # 5. VERSION CHECK
        client_version = parsed_section['runner']['robotmk_version']
        if client_version != ROBOTMK_VERSION:
            first_line.append(
                "Robotmk version mismatch (server: %s, client: %s) (!)" %
                (ROBOTMK_VERSION, client_version))
            rc = max(rc, 1)
        else:
            first_line.append("Robotmk version %s (server and client)" %
                              ROBOTMK_VERSION)

        # putting things together
        summary = ', '.join(first_line) 
        details = ''.join(out_lines) or None
        yield Result(
            state=State(rc),
            summary=summary,
            details=details           
        )
    else:
        # item is a regular s/t/k check
        for root_suite in parsed_section['suites']:
            if 'parsed' in root_suite:
                html_exists = bool(root_suite.get('htmllog'))
                if html_exists: 
                    # Discovery part see Ref #exNx1h
                    for host in runner_assigned_host: 
                        host_dir = "%s/%s" % (HTML_LOG_DIR, host)
                        save_htmllog(host_dir, "%s_last_log.html" % root_suite['id'], root_suite['htmllog'])
                        if root_suite['rc'] > 0: 
                            save_htmllog(host_dir, "%s_last_error_log.html" % root_suite['id'], root_suite['htmllog'])

                for discovered_item in root_suite['parsed'].discovered:
                    # Remove the prefix to get the original item name
                    item_without_prefix = strip_svc_prefix(item, root_suite, service_prefix)
                    if discovered_item.name == item_without_prefix:
                        now = datetime.datetime.now(tzlocal())
                        last_end = parser.isoparse(root_suite['end_time'])
                        age = now - last_end
                        if age.total_seconds() < root_suite['cache_time']:
                            for i in evaluate_robot_item(discovered_item, params_dict):
                                yield i
                        else:
                            # Keeping the following only for recalling....
                            # A stale result should not return anything here. 
                            # It's enough to have it alarmed by the Robotmk 
                            # stale monitoring check (see Ref. N2UC9N)
                            pass
                            # overdue_sec = round(
                            #     age.total_seconds() - root_suite['cache_time'],
                            #     1)
                            # yield ignore_robot_item(root_suite, last_end,
                            #                         overdue_sec)

    # We should not come here. Item cannot be found in parsed data.
    # see PRO TIP: simple return if no data is found
    # http://bit.ly/3epEcf3
    return  

def ignore_robot_item(root_suite, last_end, overdue_sec):
    # TODO: (Perhaps make this configurable (OK/UNKNOWN))
    last_end_fmt = last_end.strftime('%Y-%m-%d %H:%M:%S')
    out = "Result of suite '%s' is too old. " % root_suite['id'] + \
        "Last execution end: %s, " % last_end_fmt + \
        "overdue since %ss " % (overdue_sec) + \
        "(cache time: %ss)" % str(root_suite['cache_time'])
    return 3, out


def evaluate_robot_item(robot_item, params):
    item_result = robot_item.get_checkmk_result(robot_item, params)
    rc = item_result['worststate']
    result = Result(
        state=State(rc),
        summary=item_result['padded_lines_list'][0],
        details='\n'.join(item_result['padded_lines_list'])
    )
    # Return back a list of everything which should be yielded
    # Perfdata are generated in ref #5LSK99
    return [result] + item_result['cmk_perfdata_list']


def get_svc_prefix_tplstring(itemname, root_suite, prefix):
    '''Determines the prefix for an item as defined with pattern for root suite'''
    fmtstring = pattern_match(prefix, root_suite['parsed'].name,
                              DEFAULT_SVC_PREFIX)
    template = Template(fmtstring)
    ret_prefix = template.safe_substitute(PATH=root_suite['path'],
                                      TAG=root_suite['tag'],
                                      SUITEID=root_suite['id'],
                                      SUITENAME=root_suite['parsed'].name,
                                    #   EXEC_MODE=
                                      SPACE=' ')
    return ret_prefix


def add_svc_prefix(itemname, root_suite, prefix):
    '''Returns the item name with a templated prefix string in front of it'''
    return "%s%s" % (get_svc_prefix_tplstring(itemname, root_suite, prefix), itemname)


def strip_svc_prefix(itemname, root_suite, prefix):
    '''Strips off the templated prefix string from the front of an item name'''
    prefix_tplstring = get_svc_prefix_tplstring(itemname, root_suite, prefix)
    if itemname.startswith(prefix_tplstring):
        return itemname[len(prefix_tplstring):]
    else:
        return itemname

# ==============================================================================


class RobotItem(object):
    # maps XML tags to Classes
    class_dict = {
        'suite': 'RobotSuite',
        'test': 'RobotTest',
        'kw': 'RobotKeyword'
    }

    indentation_char = u"\u2504"

    # Indentation chars.
    # Ex.: Given a discovery level of 2 discovers tests then
    # - the test has a padding of       2-2 *-1 = 0 chars.
    # - the kw below have a padding of (2-3)*-1 = 1 chars
    @property
    def padstring(self):
        return (int(RobotItem.discovery_setting.level) -
                self.lv_abs) * -1 * self.indentation_char

    # Abbreviation for Suite/Test/Keyword - [S]/[T]/[K]
    @property
    def abbreviation(self):
        return '[%s]' % str(self)[:1].upper()

    @property
    def item_nagios_status(self):
        return self._item_nagios_status

    @item_nagios_status.setter
    def item_nagios_status(self, state):
        self._item_nagios_status = max(self._item_nagios_status, int(state))

    # Ref: r3U0Np
    def __init__(self, xmlnode, lv_abs, lv_rel, parent, index=None):
        self.xmlnode = xmlnode
        self.lv_abs = lv_abs
        self.lv_rel = lv_rel
        self.parent = parent
        self.id = self._get_id(xmlnode, index)
        if self.parent is None: 
            RobotItem.root_suite = self
            # what was discovered (depending on discovery_level)
            # Ref: yoczO3
            self.discovered = []
            # discovered messages for the Robotmk service 
            self.robotmk_messages = []


        self.status = xmlnode.find('status').attrib['status']
        self.msg = xmlnode.findtext('./msg')
        self.text = xmlnode.findtext('./status')
        if xmlnode.attrib['name'] == 'Add Monitoring Message':
            data = json.loads(self.msg)['add_monitoring_message']
            self.root_suite.robotmk_messages.append(
                data
            )

        self.name = xmlnode.attrib['name']
        self._item_nagios_status = 0
        self.elapsed_time = self._get_node_elapsed_time()
        self.result = {}
        # list containing all messages from cmk_runtime, cmk_metric of sub nodes
        self.sub_messages = []


        # Bool flag to indicate whether this is a node where messages should be added
        # (not needed for Keywords)
        self.is_topnode = False
        # relative level must be resetted on test or keyword layer
        if self.parent == None or self.parent.xpath_name != self.xpath_name:
            self.lv_rel = 0

        self.subnodes = self._get_subnodes(xmlnode)
        # Add this node if it is on the "to discover" level and if it is not blacklisted
        if self.lv_abs == int(self.discovery_setting.level):
            # Empty blacklist = inventorize all
            if self.discovery_setting.blacklist_pattern == '' or not re.match(
                    self.discovery_setting.blacklist_pattern, self.name):
                # Ref: yoczO3
                self.root_suite.discovered.append(self)

    @property
    def text(self): 
        # Return back plain text if the text has a HTML prefix.
        # This is the necessary for test messages created by rebot after merging test results. 
        # Change when this https://github.com/robotframework/robotframework/issues/4068 has been solved.         
        if self._text.startswith('*HTML* '): 
            return html_to_text(self._text).replace('*HTML* ', '')
        else: 
            return self._text

    @text.setter
    def text(self, text):
        self._text = text

    def _get_id(self, xmlnode, index):
        """suites and tests have a id attribute. Fake this for keywords.
        because indexing is important for Checkmk graphs."""
        if index != None:
            # metric index should start with 1
            return "%s-k%s" % (self.parent.id, index + 1)
        else:
            return xmlnode.attrib['id']

    # returns a list of subnode objects within a XML node
    def _get_subnodes(self, xmlnode):
        subnodes = []
        for nodetype in self.allowed_subnodes:
            for index, xmlsubnode in enumerate(xmlnode.findall(nodetype)):
                RobotClass = eval(self.class_dict[nodetype])
                node = RobotClass(xmlsubnode, self.lv_abs + 1, self.lv_rel + 1,
                                  self, index)
                subnodes.append(node)
        return subnodes

    def _get_node_elapsed_time(self):
        """Returns the time between given timestamps of a node in seconds."""
        self.start_time = self.xmlnode.find('status').attrib['starttime']
        self.end_time = self.xmlnode.find('status').attrib['endtime']
        if self.start_time == self.end_time or not (self.start_time
                                                    and self.end_time):
            return 0
        start_millis = timestamp_to_millis(self.start_time)
        end_millis = timestamp_to_millis(self.end_time)
        # start/end_millis can be long but we want to return int when possible
        return int(end_millis - start_millis) / float(1000)

    # If the pattern for a WATO <setting> matches, return the value (if tuple) or True
    def _get_pattern_value(self, setting, check_params):
        setting_keyname = getattr(self, "%s_dict_key" % setting)
        patterns = check_params.get(setting, {}).get(setting_keyname, [])
        return pattern_match(patterns, self.name)

    def _set_node_info(self):
        self.result['name'] = self.name
        self.result['abbreviation'] = self.abbreviation
        self.result['xpath_name'] = self.xpath_name

    # Evaluate the Robot status of this item to a Nagios state & set message
    def _eval_node_robotframework_status(self, check_params):
        if type(self) == RobotKeyword:
            # Keywords should only show messages if allowed by WATO rule
            if bool(check_params.get(
                    'show_kwmessages')) and not self.msg is None:
                # Playwright produces ugly log lines with lots of equal signs
                statustext = re.sub('={2,}', '==', self.msg)
                # This is to prevent Mulisite GUI to replace URLs by a unicode icon
                statustext = re.sub('http://', 'http//', statustext)
            else:
                statustext = ''
        else:
            statustext = self.text

        self.result['result_robotframework'] = (ROBOT_NAGIOS_STATUS[self.status],
                                                remove_nasty_chars(statustext))

    # create the "base line" with the node name and the RF status
    def _set_node_padded_line(self, check_params):
        # I. Begin with the baseline formatting. The baseline is pure related to the Robot result
        # ---- [K] 'MyKeyword': PASS (...)"

        # Set the message text
        text = self.result['result_robotframework'][1]
        text_bracket = ''
        if len(text) > 0:
            text_bracket = ' (%s)' % text

        # If configured, the topmost node can contain additional data:
        # - last suite execution
        # -
        endtime_str = ""
        if self.is_topnode and bool(check_params.get('includedate')):
            if self.end_time == 'N/A': 
                endtime_str = " (last execution: N/A, all retries failed)"
            else:
                try: 
                    endtime = datetime.datetime.strptime(self.end_time,
                                                        '%Y%m%d %H:%M:%S.%f')
                    endtime_str = " (last execution: %s) " % endtime.strftime(
                        '%m/%d %H:%M:%S')
                except: 
                    endtime_str = " (unknown error: cannot determine execution time)"
        baseline = ("%s %s %s '%s': %s%s%s%s" %
                    (self.padstring, '--SYMBOL--', self.abbreviation,
                     remove_nasty_chars(self.name), self.status, endtime_str,
                     '--BADGE--', text_bracket)).strip()

        # Baseline completed.
        # II. Now add results from further checks of this node (runtime, metrics, ...)
        NodeResult = namedtuple('NodeResult', 'check,resultuple')
        node_results = [
            NodeResult(check, self.result[check]) for check in self.result_keys
            if self.result.get(check, False)
        ]
        # All all messages from other node's checks; leave the one which is eventually
        # the same as the text from the result_robotframework check. (This is the case 
        # for kw_test_state.) 
        node_messages = [
            node_result.resultuple[1] for node_result in node_results
            if node_result.resultuple[1] and node_result.resultuple[1] != text
        ]

        # If this is a top_node, add the messages from subelements:
        if self.is_topnode:
            # TODO: What are examples of submessages (documentation!)
            # HEREIWAS
            if bool(check_params.get('show_submessages')):
                if len(self.sub_messages) > 0:
                    node_messages.extend(self.sub_messages)
            if len(text) > 0:
                node_messages.append(text)
            # In some cases (e.g. Set Test Message), the Node's text already got the
            # msg set by RF. Add only all others
            node_messages = [msg for msg in node_messages if msg != text]

        # III. Create submessages for the node's top_node.  (which is for kws: Test, for tests: Suite)
        messages_str = ""
        if len(node_messages) > 0:
            # But not all... We do not want messages from cmk_runtime messages if the runtime was
            # not exceeded.
            #
            # Perhaps a more generic way is needed to hinder metrics to be propagated under
            # certain conditions.

            # Add NOK runtimes and all others
            top_messages = [
                node_result.resultuple[1] for node_result in node_results
                if (node_result.resultuple[1]
                    and node_result.check != 'result_cmk_runtime') or (
                        node_result.resultuple[1] and node_result.check ==
                        'result_cmk_runtime' and node_result.resultuple[0] > 0)
            ]
            if top_messages:
                self.node_top.sub_messages.append(
                    "%s '%s': %s" %
                    (self.abbreviation, self.name, ', '.join(top_messages)))
            messages_str = ", %s" % ', '.join(node_messages)
        # Final Line
        self.result['padded_lines_list'] = ["%s%s" % (baseline, messages_str)]

    # sets status and message for this node with exceeded runtime
    # Runtime monitoring is not related to Robot Framework and introduces the WARN
    # state. Therefore, it can happen that a s/t/k is CRIT/WARN but the RF status is PASS.
    def _eval_node_cmk_runtime(self, check_params):
        runtime_threshold = self._get_pattern_value('runtime_threshold',
                                                    check_params)
        if bool(runtime_threshold):
            # CRITICAL threshold
            if self.elapsed_time >= runtime_threshold[1]:
                nagios_status = 2
                text = "%s runtime=%.2fs >= %.2fs" % (
                    STATE_BADGES[nagios_status], self.elapsed_time,
                    runtime_threshold[1])
            # WARNING threshold
            elif self.elapsed_time >= runtime_threshold[0]:
                nagios_status = 1
                text = "%s runtime=%.2fs >= %.2fs" % (
                    STATE_BADGES[nagios_status], self.elapsed_time,
                    runtime_threshold[0])
            else:
                nagios_status = 0
                if bool(
                        check_params.get('runtime_threshold',
                                         False).get('show_all_runtimes',
                                                    False)):
                    text = "runtime=%.2fs" % self.elapsed_time
                else:
                    text = ""

            cmk_runtime = (nagios_status, text)
            self.result['result_cmk_runtime'] = cmk_runtime
        else:
            self.result['result_cmk_runtime'] = None

    def _eval_node_cmk_perfdata(self, check_params):
        # Ref #5LSK99
        # PERFDATA ---- Which elemens should produce performance data?
        # this_runtime_threshold = None
        runtime_threshold = self._get_pattern_value('runtime_threshold',
                                                    check_params)
        perfdata_wanted = self._get_pattern_value('perfdata_creation',
                                                  check_params)
        if perfdata_wanted and self.elapsed_time != None:
            perflabel = get_perflabel("%s_%s" % (self.id, self.name))
            if runtime_threshold:
                cmk_perfdata = Metric(
                    perflabel, 
                    float("%.2f" % self.elapsed_time), 
                    levels=(
                        float("%.2f" % runtime_threshold[0]), 
                        float("%.2f" % runtime_threshold[0]),
                    ), 
                )
            else:
                cmk_perfdata = Metric(
                    perflabel, 
                    float("%.2f" % self.elapsed_time), 
                )
            # perfdata is a list because it gets expanded by perfdata of sub-nodes
            self.result['cmk_perfdata_list'] = [cmk_perfdata]
        else:
            self.result['cmk_perfdata_list'] = []

    # from Robotmk Keyword Library 
    # https://pypi.org/project/robotframework-robotmk/ 
    # The result of this keyword is of NO MEANING for the test. It affects the 
    # state of the Robotmk service, see Ref. 8nIZ5J
    def _eval_node_kw_robotmk_state(self):
        if self.name == 'Add Monitoring Message' and len(self.msg) > 0:
            try: 
                data = json.loads(self.msg)['add_monitoring_message']
                state = STATES_NO[data['nagios_state']]
                msg   = "%s: %s" % (data['nagios_state'], data['msg'])
                kw_robotmk_state = (0, msg)
                self.msg = msg
                # as an exception, we overwrite the msg of RF here, because we 
                # do not want to see a raw dict 
                self.result['result_robotframework'] = kw_robotmk_state
            except:
                pass

    # from Robotmk Keyword Library 
    # https://pypi.org/project/robotframework-robotmk/ 
    def _eval_node_kw_test_state(self):
        if self.name == 'Add Checkmk Test State' and len(self.msg) > 0:
            try: 
                data = json.loads(self.msg)['add_checkmk_test_state']
                state = STATES_NO[data['nagios_state']]
                msg   = "%s: %s" % (data['nagios_state'], data['msg'])
                kw_test_state = (state, msg)
                self.msg = msg
                # as an exception, we overwrite the msg of RF here, because we 
                # do not want to see a raw dict 
                self.result['result_robotframework'] = kw_test_state
            except:
                kw_test_state = None
        else:
            kw_test_state = None
        self.result['result_kw_test_state'] = kw_test_state

    # WIP: see https://github.com/simonmeggle/robotmk/issues/60
    def _eval_node_cmk_metric(self, check_params):
        #metric = self._get_pattern_value('metric', check_params)
        # TODO THIS WILL BE IMPLEMENTED
        metric = False
        if metric:
            dummy_value = randint(100, 999)
            dummy_value = 400
            dummy_name = "FOO"
            dummy_warn = 300
            dummy_crit = 600
            # CRITICAL threshold
            if dummy_value >= dummy_crit:
                nagios_status = 2
                text = "%s value %s=%s >= %s" % (STATE_BADGES[nagios_status],
                                                 dummy_name, dummy_value,
                                                 dummy_crit)
            # WARNING threshold
            elif dummy_value >= dummy_warn:
                nagios_status = 1
                text = "%s value %s=%s >= %s" % (STATE_BADGES[nagios_status],
                                                 dummy_name, dummy_value,
                                                 dummy_warn)
            else:
                nagios_status = 0
                text = "value %s=%s" % (dummy_name, dummy_value)
            # TODO: add perfdata if needed
            cmk_metric = (nagios_status, text)
            self.result['result_cmk_metric'] = cmk_metric
        else:
            cmk_metric = None

    def _descending_allowed(self, depth_limit_inherited, check_params):
        # OUTPUT DEPTH --- how deep can we descend in nested suites/keywords?
        depth_limit = self._get_pattern_value('output_depth', check_params)

        # i = inherited depth limit
        # t = this depth limit
        # nx = next depth limit

        # next_depth_limit > 0  ->  we can descend
        # next_depth_limit = 0  ->  we can descend, stop at next level
        # next_depth_limit < 0  ->  we cannot descend anymore

        # (a dot indicates a set value)
        # i  t  nx
        # -----------
        # n  n. n    # see note 3
        # n  0. -1   # see note 4
        # n  2. 1    # see note 1
        #
        # 0. n  -1   # see note 2
        # 0  0. -1   # see note 4
        # 0  2. 1    # see note 1
        #
        # 2  n  1    # see note 2
        # 2  0. -1   # see note 4
        # 2  2. 1    # see note 1

        # Now calculate the depth level for the next sub-item
        next_depth_limit = None
        if depth_limit == None or depth_limit > 0:
            if bool(depth_limit):
                # note 1: depth_limit is set to something else than 0 or None; we can descend.
                # Now calculate next depth from this level
                next_depth_limit = depth_limit - 1
            else:
                if depth_limit_inherited is None:
                    # note 3: No depth limit, no inherited limit. Set next limit also to None.
                    next_depth_limit = None
                else:
                    # note 2: No depth limit set, but inherited value. Calculate new one.
                    next_depth_limit = depth_limit_inherited - 1
        else:
            # 4 there's 0 defined, this overwrites inherited depth
            next_depth_limit = -1
        # return True if descending is allowed
        descend_allowed = next_depth_limit == None or next_depth_limit > -1
        return descend_allowed, next_depth_limit

    # This method combines the results of subnodes with the result of the current node.
    # It determines a "WORST STATE" which can be propagated
    #   - Keywords: from cmk_runtime, cmk_metric
    #     Explanation: Keywords CAN fail, but when wrapped in other keywords like
    #     'Run Keyword And Return Status', they won't break a test.
    #   - Suite, Tests: from robotframework, cmk_runtime, cmk_metric
    #     Explanation: Suites and Tests are nodes which can be services in CMK.
    #     For this reason, the state of such nodes is the worst state of the RF
    #     result and every CMK/Robotmk check (runtime, metric).
    #     Ex.: Even if a test is RF=PASS (=OK), a runtime exceedance could turn
    #     it to WARNING. Otherwise, if runtime is OK but the test FAILed, it has to
    #     be CRITICAL.

    def _eval_total_result(self):
        # NODE WORST STATE - grab all results of this node
        node_results = [
            self.result[check] for check in self.result_keys
            if self.result.get(check, False)
        ]
        if len(node_results) > 0:
            # maximum of all node result states
            node_worststate = max([x[0] for x in node_results])
        else:
            # theere is no result => OK
            node_worststate = 0
        # SUBNODES WORST STATE
        subnodes_worststate = 0
        subnodes_worststate = max(
            [s['worststate'] for s in self.subresults if self.subresults]
            or [0])
        total_worststate = max(node_worststate, subnodes_worststate)
        self.result['worststate'] = total_worststate

        # now that the worstate is known, we can replace the badge and unicode symbol placeholder
        # set the unicode symbol
        if total_worststate > 0:
            status_symbol = self.symbol_nok
        else:
            status_symbol = self.symbol_ok

        # For RF-state, do not display badges in keywords (kws are allowed to fail)
        badge = ''
        if type(self) != RobotKeyword:
            badge = STATE_BADGES[total_worststate]
            if len(badge) > 0:
                badge = " " + badge
        # The first item is the line of this node (self) which we need to edit
        this_node_paddedline = self.result['padded_lines_list'][0]
        this_node_paddedline_replaced = this_node_paddedline.replace(
            '--BADGE--', badge).replace('--SYMBOL--', status_symbol)
        self.result['padded_lines_list'][0] = this_node_paddedline_replaced

    # Add all lines of subnodes to the current one
    def _merge_sub_padded_lines(self):
        for r in self.subresults:
            for s in r['padded_lines_list']:
                self.result['padded_lines_list'].append(s)
        # sub_padded_lines_list = [s['padded_lines_list'] for s in self.subresults ]
        # self.result['padded_lines_list'].extend(sub_padded_lines_list)
        return

    # Add all perfdata of subnodes to the current one
    def _merge_sub_perfdata(self):
        for subresult in self.subresults:
            if subresult['cmk_perfdata_list'] != None:
                try:
                    self.result['cmk_perfdata_list'].extend(
                        subresult['cmk_perfdata_list'])
                except:
                    self.result['cmk_perfdata_list'] = subresult[
                        'cmk_perfdata_list']
        return

    # recursive function to retrieve status and submessages of a node
    # returns a result dict of each item node (=self)
    # node_top = the top node where messages should be reported to
    #   - sub-suites & tests: CMK item = root suite
    #   - keywords: parent test
    def get_checkmk_result(self,
                           node_top,
                           check_params,
                           depth_limit_inherited=None):
        self.node_top = node_top
        # Is node_top pointing to same node?
        if self == self.node_top:
            self.is_topnode = True
        else:
            if type(self) == RobotTest:
                self.is_topnode = True
                # for the following kws, point to this parent test
                node_top = self

        # do the recursion
        self.subresults = []
        (descend_allowed,
         next_depth_limit) = self._descending_allowed(depth_limit_inherited,
                                                      check_params)
        if descend_allowed:
            # Since RF4.0, the XML contains also keywords which would have come 
            # after a FAILed keyword (NOT RUN, SKIP). However, they are useless for Robotmk. 
            for subnode in [ i for i in self.subnodes if i.status in ROBOT_NAGIOS_STATUS.keys()]:
                subresult = subnode.get_checkmk_result(node_top, check_params,
                                                       next_depth_limit)
                self.subresults.append(subresult)

        # THIS Node -----
        self._set_node_info()
        self._eval_node_robotframework_status(check_params)
        self._eval_node_cmk_runtime(check_params)
        self._eval_node_cmk_perfdata(check_params)
        self._eval_node_kw_robotmk_state()
        self._eval_node_kw_test_state()
        self._eval_node_cmk_metric(check_params)
        # now generate the padded line incl. the message
        self._set_node_padded_line(check_params)

        # MERGE padded_lines and perfdata of sub-items
        self._merge_sub_padded_lines()
        self._merge_sub_perfdata()

        # Now that all information about this node have been collected, evaluate
        # and set the badge and unicode symbol for S/T/K
        self._eval_total_result()

        return self.result


class RobotSuite(RobotItem):
    # how to search this on the xml
    xpath_name = 'suite'
    # which subnode types are allowed
    allowed_subnodes = ['suite', 'test']
    symbol_ok = "\u25ef"
    symbol_nok = "\u2b24"
    # which key in dict output_depth is holding the values for tests
    output_depth_dict_key = "output_depth_suites"
    runtime_threshold_dict_key = "runtime_threshold_suites"
    perfdata_creation_dict_key = "perfdata_creation_suites"
    # What should be evaluated to get the node's status?
    result_keys = 'result_robotframework result_cmk_runtime result_cmk_metric'.split(
    )

    def __init__(self, xmlnode, lv_abs, lv_rel, parent, index):
        # Ref: ElI53P
        # Ref: r3U0Np
        super(RobotSuite, self).__init__(xmlnode, lv_abs, lv_rel, parent)

    def __str__(self):
        return "Suite"


class RobotTest(RobotItem):
    # how to search this on the xml
    xpath_name = 'test'
    # which subnode types are allowed
    allowed_subnodes = ['kw']
    symbol_ok = "\u25a1"
    symbol_nok = "\u25a0"
    # which key in dict output_depth is holding the values for tests
    output_depth_dict_key = "output_depth_tests"
    runtime_threshold_dict_key = "runtime_threshold_tests"
    perfdata_creation_dict_key = "perfdata_creation_tests"
    # What should be evaluated to get the node's status?
    result_keys = 'result_robotframework result_cmk_runtime result_cmk_metric'.split(
    )

    def __init__(self, xmlnode, lv_abs, lv_rel, parent, index):
        super(RobotTest, self).__init__(xmlnode, lv_abs, lv_rel, parent)

        # FIXME needed?
        # Stores the information if a test has the critical tag (only test with
        # this tag can affect the suite status)
        if xmlnode.find('status[@critical="yes"]') != None:
            self.critical = True
        else:
            self.critical = False

    def __str__(self):
        return "Test"


class RobotKeyword(RobotItem):
    # how to search this on the xml
    xpath_name = 'kw'
    # which subnode types are allowed
    allowed_subnodes = ['kw']
    symbol_ok = "\u25cb"
    symbol_nok = "\u25cf"
    # which key in dict output_depth is holding the values for keywords
    output_depth_dict_key = "output_depth_keywords"
    runtime_threshold_dict_key = "runtime_threshold_keywords"
    perfdata_creation_dict_key = "perfdata_creation_keywords"
    # What should be evaluated to get the node's status?
    result_keys = 'result_cmk_runtime result_cmk_metric result_kw_test_state'.split()

    def __init__(self, xmlnode, lv_abs, lv_rel, parent, index):
        super(RobotKeyword, self).__init__(xmlnode, lv_abs, lv_rel, parent,
                                           index)

    def __str__(self):
        return "Keyword"

def parse_suite_xml(root_xml, discovery_setting):
    # Store discovery level
    RobotItem.discovery_setting = discovery_setting
    # # clear the class var
    # RobotItem.discovered = []
    # create the topmost suite from the root XML
    # Ref: ElI53P
    root_suite = RobotSuite(root_xml, 0, 0, None, None)
    return root_suite


#   _          _
#  | |        | |
#  | |__   ___| |_ __   ___ _ __
#  | '_ \ / _ \ | '_ \ / _ \ '__|
#  | | | |  __/ | |_) |  __/ |
#  |_| |_|\___|_| .__/ \___|_|
#               | |
#               |_|

def save_htmllog(dir, logname, raw_html):
    filename = "%s/%s" % (dir, logname)
    try:
        with open(filename, 'w') as f:
            f.write(raw_html) 
    except Exception:
        raise MKGeneralException("Robotmk failed to save the log file %s" % filename)

def assign_html_logs(svc_desc, info_dict, root_suite, html_show_patterns):
    # If piggyback was not set anyway, the client tried to determine its hostname
    # and FQDN (if set) and hopefully there is a match on the Checkmk server. 
    runner_assigned_host = info_dict['runner'].get('assigned_host', [])
    svc_labels = []
    html_exists = bool(root_suite.get('htmllog'))
    if html_exists:
        # Check part see Ref #exNx1h
        for host in runner_assigned_host: 
            # There is 1 HTML log per root suite. Because of the fact that 
            # the discovery level feature of Robotmk allows to generate more 
            # than 1 service out of a root suite (example: 1 suite with 
            # test1..test5 => service test1...5), we need a translation between 
            # the discovered items (test) and the single log file (suite). 
            # /hostname
            #   $SUITE_last_error_log.html
            #   $SUITE_last_log.html
            #   /discovered_item1
            #     suite_last_error_log.html -> ../$SUITE_last_error_log.html          
            #     suite_last_log.html -> ../$SUITE_last_log.html          
            #   /discovered_item2
            #     suite_last_error_log.html -> ../$SUITE_last_error_log.html          
            #     suite_last_log.html -> ../$SUITE_last_log.html          
            for log in ['last_log', 'last_error_log']:
                pattern = html_show_patterns.get(log, '.*')
                if re.match(pattern, svc_desc): 
                    svc_dir = "%s/%s/%s" % (HTML_LOG_DIR, host, svc_desc)
                    #shutil.rmtree(svc_dir, ignore_errors=True)
                    mkdirp(svc_dir)
                    src_name = "../%s_%s.html" % (root_suite['id'], log)
                    dest_name = "%s/%s" % (svc_dir, "suite_%s.html" % log)
                    lns(src_name, dest_name)
                    svc_labels.append(ServiceLabel(u"robotmk/html_%s" % log, u"yes"))
    return svc_labels

def mkdirp(path):
    """Python2 and 3 compatible replacement for mkdir -p without raising an 
    exception if the dir already exists"""
    if not os.path.isdir(path):
        os.makedirs(path)

def lns(src, dest):
    """Python2 and 3 compatible replacement for ln -s without raising an 
    exception if the link already exists"""
    if not os.path.islink(dest):
        os.symlink(src, dest)

# create a valid perfdata label which does contain only numbers, letters,
# dash and underscore. Everything else becomes a underscore.
def get_perflabel(instr):
    outstr = re.sub('[^A-Za-z0-9]', '_', instr)
    return re.sub('_+', '_', outstr)


# Return an empty string for the string cast of None
def xstr(s):
    if s is None:
        return ''
    else:
        return s


def remove_nasty_chars(instr):
    # Replace all chars which can cause problem in Multisite
    # no quotes, no brackets
    outstr = re.sub('[\[\]?+*@{}\'"]', '', xstr(instr))
    outstr = outstr.replace('$', '')
    outstr = outstr.replace('\\', '')
    # Newlines better replace by space
    outstr = outstr.replace('\n', ' ')
    # dash for pipe
    outstr = re.sub('\|', '-', outstr)
    return outstr

# Return only fatal suites
def check_fatal_suites(suites):
    # TODO: return list of output lines
    return [s for s in suites if s['status'] == 'fatal']


# A suite can be stale in two cases:
# - A) start_time/end_time exist: the suite ran.
#   Age is "time since end_time"; stale if age > cache_time
# - B) only start_time exists: the suite is running (or: hanging forever??).
#   Age is then "time since start_time"; stale if age > cache_time
def check_stale_suites(suites):
    suites_stale = []
    suites_nonstale = []
    Suite = namedtuple('Suite', 'id,msg')
    for root_suite in [s for s in suites if s['status'] != 'fatal']:
        now = datetime.datetime.now(tzlocal())
        if 'end_time' in root_suite:
            # Case A) (suite ran)
            last_end = parser.isoparse(root_suite['end_time'])
            age = now - last_end
            last_end_fmt = last_end.strftime('%Y-%m-%d %H:%M:%S')
            if age.total_seconds() < root_suite['cache_time']:
                # nonstale
                msg = "Suite '%s': (last execution end: %s, " % (root_suite['id'], last_end_fmt)
                suites_nonstale.append(
                    Suite(
                        root_suite['id'],
                        msg
                    ))
            else:
                # stale result
                overdue_sec = age.total_seconds() - root_suite['cache_time']
                msg = "(!!) Suite '%s': (last execution end: %s, " % (root_suite['id'], last_end_fmt) + \
                    "cache time: %ds, overdue since %.1fs)\n" % (root_suite['cache_time'], overdue_sec)
                suites_stale.append(
                    Suite(
                        root_suite['id'],
                        msg
                     ))
        else:
            # Case B) (suite started, no end_time)
            last_start = parser.isoparse(root_suite['start_time'])
            age = now - last_start
            last_start_fmt = last_start.strftime('%Y-%m-%d %H:%M:%S')
            if age.total_seconds() < root_suite['cache_time']:
                # nonstale
                msg =  "Suite '%s': (started 1st time at: %s, " % (root_suite['id'], last_start_fmt)
                suites_nonstale.append(
                    Suite(
                        root_suite['id'],
                        msg
                    ))                    
            else:
                # stale result
                overdue_sec = age.total_seconds() - root_suite['cache_time']
                msg = "(!!) Suite '%s': (started 1st time at %s, " % (root_suite['id'], last_start_fmt) + \
                     "cache time: %ds, overdue since %.1fs)\n" % (root_suite['cache_time'], overdue_sec)
                suites_stale.append(
                    Suite(
                        root_suite['id'],
                        msg
                    ))
    return (suites_stale, suites_nonstale)


def quoted_listitems(inlist):
    return ', '.join(["'%s'" % s for s in inlist])


# Determine if a list of patterns matches.
# If list elements are tuples, all values from index 1
# If list elements are patterns, return bool
# If nothing matches return the default
def pattern_match(patterns, name, default=None):
    for elem in patterns:
        if type(elem) == tuple:
            if re.match(elem[0], name):
                if len(elem) == 2:
                    # only one value (2nd element) for this pattern
                    return elem[1]
                else:
                    # more than 1 value (2nd and following) for this pattern (e.g. warn/crit thresholds) => return the list
                    return elem[1:]
        else:
            if re.match(elem, name):
                return True
    return default


def timestamp_to_millis(timestamp):
    Y, M, D, h, m, s, millis = split_timestamp(timestamp)
    secs = time.mktime(datetime.datetime(Y, M, D, h, m, s).timetuple())
    return roundup(1000 * secs + millis)


def split_timestamp(timestamp):
    years = int(timestamp[:4])
    mons = int(timestamp[4:6])
    days = int(timestamp[6:8])
    hours = int(timestamp[9:11])
    mins = int(timestamp[12:14])
    secs = int(timestamp[15:17])
    millis = int(timestamp[18:21])
    return years, mons, days, hours, mins, secs, millis

def roundup(number, ndigits=0, return_type=None):
    result = round(number, ndigits)
    if not return_type:
        return_type = float if ndigits > 0 else int
    return return_type(result)


"""
HTML <-> text conversions.
http://stackoverflow.com/questions/328356/extracting-text-from-html-file-using-python
"""

def html_to_text(html):

    class _HTMLToText(HTMLParser):
        def __init__(self):
            HTMLParser.__init__(self)
            self._buf = []
            self.hide_output = False

        def handle_starttag(self, tag, attrs):
            if tag in ('p', 'br') and not self.hide_output:
                self._buf.append('\n')
            elif tag in ('script', 'style'):
                self.hide_output = True

        def handle_startendtag(self, tag, attrs):
            if tag == 'br':
                self._buf.append('\n')

        def handle_endtag(self, tag):
            if tag == 'p':
                self._buf.append('\n')
            elif tag in ('script', 'style'):
                self.hide_output = False

        def handle_data(self, text):
            if text and not self.hide_output:
                self._buf.append(re.sub(r'\s+', ' ', text))

        def handle_entityref(self, name):
            if name in name2codepoint and not self.hide_output:
                c = chr(name2codepoint[name])
                self._buf.append(c)

        def handle_charref(self, name):
            if not self.hide_output:
                n = int(name[1:], 16) if name.startswith('x') else int(name)
                self._buf.append(chr(n))

        def get_text(self):
            return re.sub(r' +', ' ', ''.join(self._buf))
                
    """
    Given a piece of HTML, return the plain text it contains.
    This handles entities and char refs, but not javascript and stylesheets.
    """
    try:
        from html.parser import HTMLParser
        from html.entities import name2codepoint
        parser = _HTMLToText()
        parser.feed(html)
        parser.close()
        return parser.get_text()
    except:  
        # on Checkmk1 there is no HTML.Parser; :-/ 
        return html

# %%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%
# V2 specific functions



# v2register
register.check_plugin(
    name="robotmk",
    service_name="%s",
    discovery_function=discover_robotmk,
    discovery_ruleset_name='inventory_robotmk_rules',
    discovery_ruleset_type=register.RuleSetType.MERGED,   
    discovery_default_parameters={}, 
    check_function=check_robotmk,
    # TODO: https://docs.checkmk.com/master/de/devel_check_plugins.html#_verwenden_von_vorhandenen_regelketten
    check_ruleset_name="robotmk",
    check_default_parameters={},
)

