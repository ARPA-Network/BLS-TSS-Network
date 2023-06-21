import os.path

from aws_cdk.aws_s3_assets import Asset

from aws_cdk import aws_ec2 as ec2, aws_iam as iam, App, Stack, CfnOutput

from constructs import Construct


dirname = os.path.dirname(__file__)


class EC2InstanceStack(Stack):
    def __init__(self, scope: Construct, id: str, **kwargs) -> None:
        super().__init__(scope, id, **kwargs)

        # VPC
        vpc = ec2.Vpc(
            self,
            "VPC",
            nat_gateways=0,
            subnet_configuration=[
                ec2.SubnetConfiguration(
                    name="public", subnet_type=ec2.SubnetType.PUBLIC
                )
            ],
        )

        # AMI
        amzn_linux = ec2.MachineImage.latest_amazon_linux2(
            edition=ec2.AmazonLinuxEdition.STANDARD,
            virtualization=ec2.AmazonLinuxVirt.HVM,
            storage=ec2.AmazonLinuxStorage.GENERAL_PURPOSE,
        )

        # Instance Role and SSM Managed Policy
        role = iam.Role(
            self, "InstanceSSM", assumed_by=iam.ServicePrincipal("ec2.amazonaws.com")
        )

        role.add_managed_policy(
            iam.ManagedPolicy.from_aws_managed_policy_name(
                "AmazonSSMManagedInstanceCore"
            )
        )

        # Security group for Node-test
        sg_node_test = ec2.SecurityGroup(
            self,
            "SGNodeTest",
            vpc=vpc,
            allow_all_outbound=True,
            description="Security group for Node-test instance",
        )

        sg_node_test.add_ingress_rule(
            ec2.Peer.any_ipv4(), ec2.Port.tcp_range(50060, 50069)
        )
        sg_node_test.add_ingress_rule(
            ec2.Peer.any_ipv4(), ec2.Port.tcp_range(50090, 50099)
        )

        # Node-test Instance
        node_test = ec2.Instance(
            self,
            "NodeTest",
            instance_type=ec2.InstanceType("t2.large"),
            machine_image=amzn_linux,
            vpc=vpc,
            role=role,
            security_group=sg_node_test,
            instance_name="node-test",
            block_devices=[
                ec2.BlockDevice(
                    device_name="/dev/xvda", volume=ec2.BlockDeviceVolume.ebs(30)
                )
            ],
        )

        # Userdata scripts
        node_asset = Asset(
            self, "NodeAsset", path=os.path.join(dirname, "configure_node.sh")
        )

        node_local_path = node_test.user_data.add_s3_download_command(
            bucket=node_asset.bucket, bucket_key=node_asset.s3_object_key
        )

        # Execute userdata scripts
        node_test.user_data.add_execute_file_command(file_path=node_local_path)

        node_asset.grant_read(node_test.role)

        # Useful Outputs
        useful_outputs = CfnOutput(
            self,
            "UsefulOutputs",
            value=f"""
                Node ec2: {node_test.instance_public_ip}
                aws ssm start-session --target {node_test.instance_id}

                Env variables for contract deployment:
                export NODE_RPC_IP="{node_test.instance_public_ip}"
                export ETH_RPC_URL=""
            """,
            description="Useful Outputs",
        )


app = App()
EC2InstanceStack(app, "randcast-ec2-stack")

app.synth()
