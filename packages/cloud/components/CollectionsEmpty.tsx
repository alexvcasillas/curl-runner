import { external, routes } from '@/lib/routes';
import { ButtonLink } from './ButtonLink';
import { Card, CardBody, CardFooter, CardTitle } from './Card';

function Code({ children }: { children: React.ReactNode }) {
  return (
    <pre className="m-0 overflow-x-auto rounded-[6px] border border-hairline bg-raised px-3.5 py-3 font-mono text-[12px] leading-[18px] text-fg">
      {children}
    </pre>
  );
}

const K = ({ children }: { children: React.ReactNode }) => <span>{children}</span>;
const S = ({ children }: { children: React.ReactNode }) => (
  <span className="text-success">{children}</span>
);

const TOKEN_SECRET = ['$', '{{ secrets.CURL_RUNNER_TOKEN }}'].join('');

/** No collections yet: three steps from a config file to CI. */
export function CollectionsEmpty({ team }: { team: string }) {
  return (
    <Card aria-labelledby="first-collection">
      <CardBody className="gap-6 p-8 max-sm:p-5">
        <div className="flex max-w-[560px] flex-col gap-1.5">
          <CardTitle id="first-collection" className="text-[20px] leading-7">
            Publish your first collection
          </CardTitle>
          <p className="m-0 text-fg-muted">
            Collections arrive here from CI. Three steps, and every push to main publishes a new
            version.
          </p>
        </div>
        <div className="grid grid-cols-[repeat(auto-fit,minmax(min(100%,300px),1fr))] gap-4">
          <div className="flex min-w-0 flex-col gap-2.5">
            <span className="font-medium">1. Describe it in curl-runner.yaml</span>
            <Code>
              <K>cloud:</K>
              {'\n  '}
              <K>collection:</K> <S>payments-api</S>
              {'\n  '}
              <K>name:</K> <S>Payments API</S>
            </Code>
          </div>
          <div className="flex min-w-0 flex-col gap-2.5">
            <span className="font-medium">2. Make a publish token</span>
            <p className="m-0 text-[12px] leading-[18px] text-fg-muted">
              Save it as <span className="font-mono text-fg">CURL_RUNNER_TOKEN</span> in your
              repository secrets.
            </p>
            <ButtonLink
              href={routes.teamTab(team, 'tokens')}
              variant="primary"
              className="self-start"
            >
              Create token
            </ButtonLink>
          </div>
          <div className="flex min-w-0 flex-col gap-2.5">
            <span className="font-medium">3. Add the GitHub Action</span>
            <Code>
              <K>- uses:</K> <S>alexvcasillas/curl-runner/actions/publish@main</S>
              {'\n  '}
              <K>with:</K>
              {'\n    '}
              <K>token:</K> <S>{TOKEN_SECRET}</S>
              {'\n    '}
              <K>team:</K> <S>{team}</S>
            </Code>
          </div>
        </div>
      </CardBody>
      <CardFooter>
        <span className="min-w-0 flex-1">
          Not on GitHub? Run <span className="font-mono text-fg">curl-runner publish</span> from any
          CI with the same token.
        </span>
        <a href={external.cloudDocs} target="_blank" rel="noreferrer">
          Read the guide
        </a>
      </CardFooter>
    </Card>
  );
}
