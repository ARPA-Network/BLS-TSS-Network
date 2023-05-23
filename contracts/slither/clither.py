import argparse
import json
from collections import defaultdict
from cmd import Cmd


class FindingsCLI(Cmd):
    def __init__(self, findings_by_impact):
        super(FindingsCLI, self).__init__()
        self.prompt = "∴ "
        self.findings_by_impact = findings_by_impact

    def do_count(self, _):
        """List number of findings by impact."""
        self.count_findings(self.findings_by_impact)
        print()

    def do_list(self, impact):
        """List findings with a specific impact level.
        Usage: impact [high|medium|low|informational|optimization]
        """
        if impact == None or impact not in self.findings_by_impact.keys():
            print("Please provide a valid impact level.")
            return
        self.print_findings(impact.lower(), self.findings_by_impact)
        print()

    def do_detail(self, args):
        """Display the full details of a specific finding by its number in the impact list.
        Usage: detail [impact] [number]
        """
        if args.count(" ") != 1:
            print("Please provide impact and a valid finding number.")
        else:
            impact, number = args.lower().split()
            if impact not in self.findings_by_impact.keys():
                print("Please provide a valid impact level.")
                return
            try:
                index = int(number) - 1
                finding = self.findings_by_impact[impact][index]
                display_finding(finding)
            except (IndexError, ValueError):
                print("Please provide a valid finding number.")
        print()

    def do_exit(self, _):
        """Exit the Findings CLI."""
        print("Exiting...")
        return True

    def do_EOF(self, _):
        """Exit the Findings CLI using Ctrl-D."""
        print("Exiting...")
        return True

    def print_findings(self, impact, findings_by_impact):
        ("--------------------")
        print(f"Finding Impact: {impact}")
        print(f"Number of Findings: {len(findings_by_impact[impact])}")

        counter = 0

        for f in findings_by_impact[impact]:
            counter += 1
            print(f"{counter}. {f['check']}")
            print(f["description"])
            # print reference: Reference: https://github.com/crytic/slither/wiki/Detector-Documentation#reentrancy-vulnerabilities-2

    def count_findings(self, findings_by_impact):
        print("# of Findings by Impact")
        # Determine the length of the longest impact string
        longest_impact_length = max(len(impact) for impact in findings_by_impact.keys())
        # Print findings
        for impact in findings_by_impact.keys():
            print(
                f" {impact.capitalize():{longest_impact_length}}  {len(findings_by_impact[impact])}"
            )

    def do_sum(self, impact):
        """Summarize the number of findings with the specified impact grouped by check type.
        Usage: summarize [high|medium|low|informational|optimization]
        """
        if impact not in self.findings_by_impact.keys():
            print("Please provide a valid impact level.")
            return
        self.summarize_findings_by_check(impact, self.findings_by_impact)
        print()

    def summarize_findings_by_check(self, impact, findings_by_impact):
        # print(f"Summary of Findings for Impact Level: {impact}")
        summary = defaultdict(int)
        for finding in findings_by_impact[impact]:
            summary[finding["check"]] += 1

        for check, count in summary.items():
            print(f"{check:<25} {count}")


def parse_report(filename, impact_filter=None):
    with open(filename, "r") as file:
        json_data = file.read()

    data = json.loads(json_data)

    # sort findings by impact
    findings = defaultdict(list)
    for detector in data["results"]["detectors"]:
        if impact_filter:
            if detector["impact"].lower() not in impact_filter.lower().split(","):
                continue
        findings[detector["impact"].lower()].append(detector)

    return findings


def display_finding(finding):
    print(f"Finding ID: {finding['id']}")
    print(f"Finding Impact: {finding['impact']}")
    print(f"Finding Confidence: {finding['confidence']}")
    print(f"Finding Description: {finding['description']}")
    print(f"Finding Check: {finding['check']}")
    print(f"Finding Markdown: {finding['markdown']}")
    print(f"Finding First Markdown Element: {finding['first_markdown_element']}")
    print(f"Finding Elements: {finding['elements']}")


def print_instructions(args):
    print(
        """ _______  _       __________________          _______  _______
(  ____ \\( \\      \\__   __/\\__   __/|\\     /|(  ____ \\(  ____ )
| (    \\/| (         ) (      ) (   | )   ( || (    \\/| (    )|
| |      | |         | |      | |   | (___) || (__    | (____)|
| |      | |         | |      | |   |  ___  ||  __)   |     __)
| |      | |         | |      | |   | (   ) || (      | (\\ (
| (____/\\| (____/\\___) (___   | |   | )   ( || (____/\\| ) \\ \\__
(_______/(_______/\\_______/   )_(   |/     \\|(_______/|/   \\__/
                                                               """
    )
    print(f"Loaded Slither Output: {args.filename}\n")
    print("Available Commands:")
    print(f"  - {'count':<25} list finding summary")
    print(f"  - {'sum [impact]':<25} summarize findings by detector")
    print(
        f"  - {'list [impact]':<25} list findings by impact [high|medium|low|informational|optimization]"
    )
    print(f"  - {'detail [impact] [number]':<25} display full findings details")
    print(
        "\nVulnerability / Remediation Info: https://github.com/crytic/slither/wiki/Detector-Documentation"
    )


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("filename", help="JSON filename")
    parser.add_argument("-i", "--impact", help="impact level filter (comma-separated)")
    args = parser.parse_args()

    findings_by_impact = parse_report(args.filename, args.impact)

    # Run the CLI
    print_instructions(args)
    FindingsCLI(findings_by_impact).cmdloop()


if __name__ == "__main__":
    main()
